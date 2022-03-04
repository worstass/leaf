import Foundation

//public protocol CallbackInterface {
//    func reportTraffic(txRate: Float, rxRate: Float, txTotal: UInt64, rxTotal: UInt64)
//}

//func reportTraffic(txRate: Float, rxRate: Float, txTotal: UInt64, rxTotal: UInt64) {}

typealias ReportTraffic = @convention(c) (Float, Float,  UInt64,  UInt64)->Void
//
//
//public struct Callback {
//    var reportTraffic: ReportTraffic?
//}

public func runWithOptions(
    rtId: UInt16,
    configPath: String,
    callback: UnsafePointer<Callback>,
    autoReload: Bool,
    multiThread: Bool,
    autoTheads: Bool,
    threads: Int32,
    stackSize: Int32
) -> Int32 {
    return leaf_run_with_options(rtId, configPath.cString(using: .utf8), callback, autoReload, multiThread, autoTheads, threads, stackSize);
}

public func run(rtId: UInt16, configPath: String, callback: UnsafePointer<Callback>) -> Int32 {
    return leaf_run(rtId, configPath.cString(using: .utf8), callback)
}

public func reload(rtId: UInt16) -> Int32 {
    return leaf_reload(rtId)
}

public func shutdown(rtId: UInt16) -> Bool {
    return leaf_shutdown(rtId)
}
