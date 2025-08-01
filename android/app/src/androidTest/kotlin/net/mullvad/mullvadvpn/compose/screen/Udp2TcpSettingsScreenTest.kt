package net.mullvad.mullvadvpn.compose.screen

import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.performClick
import de.mannodermaus.junit5.compose.ComposeContext
import io.mockk.coVerify
import io.mockk.mockk
import net.mullvad.mullvadvpn.compose.createEdgeToEdgeComposeExtension
import net.mullvad.mullvadvpn.compose.setContentWithTheme
import net.mullvad.mullvadvpn.compose.state.Udp2TcpSettingsUiState
import net.mullvad.mullvadvpn.lib.model.Constraint
import net.mullvad.mullvadvpn.lib.model.Port
import net.mullvad.mullvadvpn.lib.ui.tag.UDP_OVER_TCP_PORT_ITEM_X_TEST_TAG
import net.mullvad.mullvadvpn.onNodeWithTagAndText
import net.mullvad.mullvadvpn.util.Lc
import net.mullvad.mullvadvpn.util.toLc
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.extension.RegisterExtension

@OptIn(ExperimentalTestApi::class)
class Udp2TcpSettingsScreenTest {
    @JvmField @RegisterExtension val composeExtension = createEdgeToEdgeComposeExtension()

    private fun ComposeContext.initScreen(
        state: Lc<Unit, Udp2TcpSettingsUiState>,
        onObfuscationPortSelected: (Constraint<Port>) -> Unit = {},
        navigateUdp2TcpInfo: () -> Unit = {},
        onBackClick: () -> Unit = {},
    ) {
        setContentWithTheme {
            Udp2TcpSettingsScreen(
                state = state,
                onObfuscationPortSelected = onObfuscationPortSelected,
                navigateUdp2TcpInfo = navigateUdp2TcpInfo,
                onBackClick = onBackClick,
            )
        }
    }

    @Test
    fun testSelectTcpOverUdpPortOption() =
        composeExtension.use {
            // Arrange
            val onObfuscationPortSelected: (Constraint<Port>) -> Unit = mockk(relaxed = true)
            initScreen(
                state = Udp2TcpSettingsUiState(port = Constraint.Any).toLc(),
                onObfuscationPortSelected = onObfuscationPortSelected,
            )

            // Act
            onNodeWithTagAndText(
                    testTag = String.format(UDP_OVER_TCP_PORT_ITEM_X_TEST_TAG, 5001),
                    text = "5001",
                )
                .assertExists()
                .performClick()

            // Assert
            coVerify(exactly = 1) { onObfuscationPortSelected.invoke(Constraint.Only(Port(5001))) }
        }
}
