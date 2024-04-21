//
//  CastsGrpcClient.swift
//  play
//
//  Created by Duane Bailey on 4/9/24.
//

import Foundation
import SwiftProtobuf


class CastsGrpcClient {
    static let shared = CastsGrpcClient()
}

extension GrpcInterface_Feed: Identifiable {
    public var id: String { feedID }
}

func toTimestamp(date: Date) -> SwiftProtobuf.Google_Protobuf_Timestamp {
    SwiftProtobuf.Google_Protobuf_Timestamp(date: date)
}
