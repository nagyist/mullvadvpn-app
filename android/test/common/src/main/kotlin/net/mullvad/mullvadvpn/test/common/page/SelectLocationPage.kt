package net.mullvad.mullvadvpn.test.common.page

import androidx.test.uiautomator.By
import net.mullvad.mullvadvpn.lib.ui.tag.EXPAND_BUTTON_TEST_TAG
import net.mullvad.mullvadvpn.lib.ui.tag.GEOLOCATION_NAME_TAG
import net.mullvad.mullvadvpn.lib.ui.tag.SELECT_LOCATION_SCREEN_TEST_TAG
import net.mullvad.mullvadvpn.test.common.extension.findObjectWithTimeout

class SelectLocationPage internal constructor() : Page() {
    override fun assertIsDisplayed() {
        uiDevice.findObjectWithTimeout(By.res(SELECT_LOCATION_SCREEN_TEST_TAG))
    }

    fun clickLocationExpandButton(locationName: String) {
        val locationCell =
            uiDevice
                .findObjectWithTimeout(By.textContains(locationName).res(GEOLOCATION_NAME_TAG))
                .parent
                .parent
        val expandButton = locationCell.findObjectWithTimeout(By.res(EXPAND_BUTTON_TEST_TAG))
        expandButton.click()
    }

    fun clickLocationCell(locationName: String) {
        uiDevice.findObjectWithTimeout(By.text(locationName)).click()
    }
}
