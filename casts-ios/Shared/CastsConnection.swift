import Foundation
import GRPC
import Logging
import NIOCore
import NIOPosix
import NIOSSL
import Security

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
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else {
            throw ConnectionError.missingCredential(account)
        }
        return [UInt8](data)
    }

    /// Build the client, authenticating with the certificate in the keychain.
    static func makeClient(group: EventLoopGroup, logger: Logger) throws
        -> GrpcInterface_CastsNIOClient
    {
        guard useTLS else {
            // Local development against a plaintext server.
            let channel = ClientConnection
                .insecure(group: group)
                .withBackgroundActivityLogger(logger)
                .connect(host: host, port: port)
            return GrpcInterface_CastsNIOClient(channel: channel)
        }

        let caBytes = try keychainPEM(account: "ca")
        let certBytes = try keychainPEM(account: "client-cert")
        let keyBytes = try keychainPEM(account: "client-key")

        let ca: NIOSSLCertificate
        let cert: NIOSSLCertificate
        let key: NIOSSLPrivateKey
        do {
            ca = try NIOSSLCertificate(bytes: caBytes, format: .pem)
            cert = try NIOSSLCertificate(bytes: certBytes, format: .pem)
            key = try NIOSSLPrivateKey(bytes: keyBytes, format: .pem)
        } catch {
            throw ConnectionError.badCredential("credential", error)
        }

        let channel = ClientConnection
            .usingTLSBackedByNIOSSL(on: group)
            // The server's certificate is issued by a private CA, so the
            // system trust store cannot verify it.
            .withTLS(trustRoots: .certificates([ca]))
            .withTLS(certificateChain: [cert])
            .withTLS(privateKey: key)
            // Without a client certificate the server does not refuse
            // cleanly: TLS 1.3 completes the handshake and the connection is
            // torn down afterwards, so a credential mistake would otherwise
            // present as an indefinite hang rather than an error.
            .withConnectionTimeout(minimum: .seconds(10))
            .withBackgroundActivityLogger(logger)
            .connect(host: host, port: port)

        return GrpcInterface_CastsNIOClient(channel: channel)
    }
}
