// Copyright (c) 2023 by Duane R. Bailey. All rights reserved.


import Foundation
import Apollo
import casts_c

extension GraphQLUnownedBuffer {
    func data() -> Data {
        Data(bytes: self.buffer, count: Int(self.length))
    }
}

extension String {
    func toGraphQLUnownedBuffer() -> GraphQLUnownedBuffer {
        let u8str = self.utf8CString;
        let byteCount = u8str.count - 1;
        let buf = UnsafeMutableRawPointer.allocate(byteCount: byteCount, alignment: 4);
        u8str.withUnsafeBytes { body in
            buf.copyMemory(from: body.baseAddress!, byteCount: byteCount)
        };
        return GraphQLUnownedBuffer(
            buffer: buf,
            length: UInt(byteCount)
        );
    }
}


func performCallback(resultBuf: GraphQLUnownedBuffer, context: UnsafeRawPointer?) -> Void {
    let callback: EmbeddedCancellableOperation = Unmanaged.fromOpaque(context!).takeRetainedValue()
    callback.callback(resultBuf.data())
}

protocol EmbeddedCallbackOperation {
    func callback(_ resultBuf: Data) -> Void;
}

class EmbeddedCancellableOperation: Cancellable, EmbeddedCallbackOperation {
    private let requestId: UUID
    private let operation: any GraphQLOperation
    private let callbackQueue: DispatchQueue
    private let completionHandler: (Result<JSONObject, Error>) -> Void
    
    init(operation: any GraphQLOperation, callbackQueue: DispatchQueue, completionHandler: @escaping (Result<JSONObject, Error>) -> Void) {
        self.requestId = UUID.init()
        self.operation = operation
        self.callbackQueue = callbackQueue
        self.completionHandler = completionHandler
    }
    
    func callback(_ resultBuf: Data) {
        do {
            let json = try JSONSerialization.jsonObject(with: resultBuf) as! JSONObject
            self.callbackQueue.async {
                self.completionHandler(.success(json))
            }
        } catch {
            print("caught error, swallowing: \(error)")
        }
    }
    
    
    func cancel() {
        print("cancel called on the cancellable")
    }
}

class EmbeddedNetworkTransport: NetworkTransport {
    private var requestCache: [UUID:any EmbeddedCallbackOperation] = [:]
    
    func send<Operation: GraphQLOperation>(
        operation: Operation,
        cachePolicy: Apollo.CachePolicy,
        contextIdentifier: UUID?,
        callbackQueue: DispatchQueue,
        completionHandler: @escaping (
            Result<Apollo.GraphQLResult<Operation.Data>, Error>
        ) -> Void
    ) -> Apollo.Cancellable where Operation : ApolloAPI.GraphQLOperation {
        let requestId = UUID()
        
        var serializedDoc: GraphQLUnownedBuffer? = nil
        switch Operation.document {
        case let .notPersisted(definition):
            serializedDoc = definition.queryDocument.toGraphQLUnownedBuffer()
        default:
            fatalError("not implemented")
        };
        

        let operation = EmbeddedCancellableOperation(operation: operation, callbackQueue: callbackQueue) { result in
            switch result {
            case let .success(json):
                do {
                    let response = GraphQLResponse(operation: operation, body: json)
                    let (result, _parsedResults) = try response.parseResult()
                    completionHandler(.success(result))
                } catch {
                    print("Error: \(error)")
                }
            default:
                fatalError("unimplemented")
            }
        }
        let retained = Unmanaged.passRetained(operation).toOpaque()
        let valuePtr = UnsafeMutablePointer<GraphQLUnownedBuffer>.allocate(capacity: 1)
        valuePtr.pointee = "34c3c08d-0d55-4af4-a126-170c1380f07e".toGraphQLUnownedBuffer()
        let inputValuePtr = UnsafeMutablePointer<GraphQLInputValue>.allocate(capacity: 1)
        inputValuePtr.pointee = GraphQLInputValue(typ: 1, count: 0, slot_b: valuePtr, slot_c: nil)
        let namePtr = UnsafeMutablePointer<GraphQLUnownedBuffer>.allocate(capacity: 1)
        namePtr.pointee = "streamId".toGraphQLUnownedBuffer()
        
        gql_casts(serializedDoc!,
                  GraphQLUnownedBuffer(buffer: nil, length: 0),
                  nil,
                  retained,
                  performCallback);
        
        return operation
    }
}


public final class EmbeddedClient {
    public static let shared = EmbeddedClient()
    public private(set) lazy var apollo = ApolloClient(networkTransport: EmbeddedNetworkTransport(), store: ApolloStore())
}

