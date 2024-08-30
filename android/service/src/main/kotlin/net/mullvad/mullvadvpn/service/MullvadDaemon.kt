package net.mullvad.mullvadvpn.service

import net.mullvad.mullvadvpn.lib.endpoint.ApiEndpointOverride

object MullvadDaemon {
    init {
        System.loadLibrary("mullvad_jni")
    }

    external fun initialize(
        vpnService: MullvadVpnService,
        rpcSocketPath: String,
        filesDirectory: String,
        cacheDirectory: String,
        apiEndpointOverride: ApiEndpointOverride?,
    )

    external fun shutdown()
}
