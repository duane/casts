import Foundation
import GRPC
import Logging
import NIOCore
import NIOPosix
import NIOSSL
import os
typealias OSLogger = os.Logger
import Security

/// Structured, timestamped logging for connection setup and every RPC.
///
/// grpc-swift's own `Logger` (passed as `withBackgroundActivityLogger`) logs
/// transport-level events, but nothing here logged *application*-level
/// events -- which Keychain item was being read, how long it took, when a
/// call started versus when it returned. A "sporadic hang" report is
/// otherwise undiagnosable after the fact: by the time it is noticed, the
/// only evidence is that the UI never updated. These are OSLog (`os.Logger`)
/// rather than print/StreamLogHandler so they show up in Console.app and
/// `log stream`, correlated with system events (sleep/wake, network changes)
/// on the same timeline -- exactly what a network-related hang needs.
enum CastsLog {
    static let connection = OSLogger(subsystem: "cc.duane.casts", category: "connection")
    static let rpc = OSLogger(subsystem: "cc.duane.casts", category: "rpc")
}

/// Logs every connectivity state transition (idle/connecting/ready/
/// transientFailure/shutdown).
///
/// This is the single most useful signal for a hang: it distinguishes a
/// connection that is `.ready` the whole time (the call itself is stuck --
/// a server or deadline problem) from one cycling through
/// `.transientFailure` and back (the client already knows the network is
/// bad and is retrying, but nothing was telling the user that).
final class CastsConnectivityLogger: ConnectivityStateDelegate {
    func connectivityStateDidChange(from oldState: ConnectivityState, to newState: ConnectivityState) {
        CastsLog.connection.notice("connectivity: \(String(describing: oldState)) -> \(String(describing: newState))")
    }
}

/// Where the app connects, and how it authenticates.
///
/// The connection used to be built twice -- once in the iOS delegate and once
/// in the macOS one -- as `.insecure(...)` against `127.0.0.1:8243`. It now
/// lives here so both platforms share one definition, and so that changing the
/// endpoint or the credentials is a one-line change rather than a hunt.
enum CastsConnection {
    /// The deployed service. Overridable for local development by setting
    /// `CASTS_HOST` / `CASTS_PORT` in the scheme's environment.
    static var host: String {
        ProcessInfo.processInfo.environment["CASTS_HOST"] ?? "casts.duane.cc"
    }

    static var port: Int {
        ProcessInfo.processInfo.environment["CASTS_PORT"].flatMap(Int.init) ?? 443
    }

    /// A local plaintext server needs no credentials; anything else does.
    static var useTLS: Bool {
        ProcessInfo.processInfo.environment["CASTS_INSECURE"] == nil
    }

    /// Keychain service name under which the three PEMs are stored.
    static let keychainService = "cc.duane.casts"

    enum ConnectionError: Error, CustomStringConvertible {
        case missingCredential(String)
        case badCredential(String, Error)

        var description: String {
            switch self {
            case let .missingCredential(account):
                return """
                No \(account) found in the keychain under service \
                "\(CastsConnection.keychainService)". casts requires a client \
                certificate; see scripts/import-casts-credentials.sh.
                """
            case let .badCredential(account, error):
                return "The \(account) in the keychain could not be parsed: \(error)"
            }
        }
    }

    /// Read one PEM from the keychain.
    ///
    /// Stored as a generic password rather than as a `SecIdentity`: NIOSSL is
    /// backed by BoringSSL and takes certificate and key bytes directly, so it
    /// cannot consume a `SecIdentity` from Security.framework.
    private static func keychainPEM(account: String) throws -> [UInt8] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        let start = DispatchTime.now()
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        let elapsedMs = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
        // A slow read here (not milliseconds, but seconds) is the signature
        // of a Keychain authorization prompt: SecItemCopyMatching blocks
        // synchronously until the user responds to it, and on an ad-hoc
        // "Sign to Run Locally" build the app's signature -- and therefore
        // its Keychain ACL identity -- changes on every rebuild, so macOS
        // re-prompts every time a freshly built binary runs. Logged, never
        // the certificate/key bytes themselves.
        guard status == errSecSuccess, let data = item as? Data else {
            CastsLog.connection.error("keychain read failed: account=\(account) status=\(status) elapsedMs=\(elapsedMs, privacy: .public)")
            throw ConnectionError.missingCredential(account)
        }
        CastsLog.connection.notice("keychain read ok: account=\(account) bytes=\(data.count) elapsedMs=\(elapsedMs, privacy: .public)")
        return [UInt8](data)
    }

    /// Build the client, authenticating with the certificate in the keychain.
    static func makeClient(group: EventLoopGroup, logger: Logging.Logger) throws
        -> GrpcInterface_CastsNIOClient
    {
        guard useTLS else {
            // Local development against a plaintext server.
            let channel = ClientConnection
                .insecure(group: group)
                .withKeepalive(Self.keepalive)
                .withConnectivityStateDelegate(CastsConnectivityLogger())
                .withBackgroundActivityLogger(logger)
                .connect(host: host, port: port)
            return GrpcInterface_CastsNIOClient(channel: channel)
        }

        let caBytes = try keychainPEM(account: "ca")
        let certBytes = try keychainPEM(account: "client-cert")
        let keyBytes = try keychainPEM(account: "client-key")

        // The "ca" item must hold the casts intermediate *and* the root,
        // concatenated (pki/dist/casts-ca-chain.cert.pem). A bare
        // intermediate looked correct -- it is enough for the server's own
        // mTLS verification, and grpcurl accepts it too -- but BoringSSL
        // (what swift-nio-ssl uses) requires trust roots to chain to a
        // self-signed certificate before completing path validation; a lone
        // intermediate fails with CERTIFICATE_VERIFY_FAILED. `openssl
        // s_client` reproduces this exactly: it fails against the bare
        // intermediate and succeeds once `-partial_chain` is added. NIOSSL
        // exposes no equivalent flag, so the chain is supplied instead.
        // `fromPEMBytes` parses every certificate block in the file, so this
        // also degrades gracefully if the item ever holds just one cert.
        let caCerts: [NIOSSLCertificate]
        let cert: NIOSSLCertificate
        let key: NIOSSLPrivateKey
        do {
            caCerts = try NIOSSLCertificate.fromPEMBytes(caBytes)
            cert = try NIOSSLCertificate(bytes: certBytes, format: .pem)
            key = try NIOSSLPrivateKey(bytes: keyBytes, format: .pem)
        } catch {
            throw ConnectionError.badCredential("credential", error)
        }

        let channel = ClientConnection
            .usingTLSBackedByNIOSSL(on: group)
            // The server's certificate is issued by a private CA, so the
            // system trust store cannot verify it.
            .withTLS(trustRoots: .certificates(caCerts))
            .withTLS(certificateChain: [cert])
            .withTLS(privateKey: key)
            // Without a client certificate the server does not refuse
            // cleanly: TLS 1.3 completes the handshake and the connection is
            // torn down afterwards, so a credential mistake would otherwise
            // present as an indefinite hang rather than an error.
            .withConnectionTimeout(minimum: .seconds(10))
            .withKeepalive(Self.keepalive)
            .withConnectivityStateDelegate(CastsConnectivityLogger())
            .withBackgroundActivityLogger(logger)
            .connect(host: host, port: port)

        return GrpcInterface_CastsNIOClient(channel: channel)
    }

    /// Detect a connection that has silently died -- after the laptop
    /// sleeps, a network switch, or a VPN reconnect -- instead of accepting
    /// it as `.ready` and only discovering it is dead when an RPC is sent on
    /// it and never returns. `permitWithoutCalls: true` is the part that
    /// matters: the default only pings while a call is in flight, which
    /// means an idle connection (the common case -- nothing is fetching
    /// most of the time) would go completely unmonitored between one call
    /// and the next.
    private static let keepalive = ClientConnectionKeepalive(
        interval: .seconds(30),
        timeout: .seconds(10),
        permitWithoutCalls: true
    )

    /// Deadline for a call, so a stuck RPC surfaces as a visible, retryable
    /// error instead of the UI silently waiting forever. Without this, the
    /// grpc-swift default (`CallOptions.timeLimit == .none`) means a request
    /// sent on a connection that looks `.ready` but is actually dead simply
    /// never completes -- indistinguishable, from the user's side, from the
    /// app having hung.
    static func callOptions(timeout: TimeAmount = .seconds(20)) -> CallOptions {
        var options = CallOptions()
        options.timeLimit = .timeout(timeout)
        return options
    }
}
