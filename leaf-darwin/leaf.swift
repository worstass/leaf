import Foundation
import Leaf_Private

public func runWithOptions(
    rtId: UInt16,
    configPath: String,
    autoReload: Bool,
    multiThread: Bool,
    autoTheads: Bool,
    threads: Int32,
    stackSize: Int32
) -> Int32 {
    return leaf_run_with_options(rtId, configPath.cString(using: .utf8), autoReload, multiThread, autoTheads, threads, stackSize);
}

public func run(rtId: UInt16, configPath: String) -> Int32 {
    return leaf_run(rtId, configPath.cString(using: .utf8))
}

public func reload(rtId: UInt16) -> Int32 {
    return leaf_reload(rtId)
}

public func shutdown(rtId: UInt16) -> Bool {
    return leaf_shutdown(rtId)
}
