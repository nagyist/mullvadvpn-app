package net.mullvad.mullvadvpn.compose.screen

import androidx.compose.material.ExperimentalMaterialApi
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performTextInput
import io.mockk.MockKAnnotations
import io.mockk.mockk
import io.mockk.verify
import io.mockk.verifyAll
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import net.mullvad.mullvadvpn.compose.state.VpnSettingsUiState
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_LAST_ITEM_TEST_TAG
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_QUANTUM_ITEM_OFF_TEST_TAG
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_QUANTUM_ITEM_ON_TEST_TAG
import net.mullvad.mullvadvpn.compose.test.LAZY_LIST_TEST_TAG
import net.mullvad.mullvadvpn.model.QuantumResistantState
import net.mullvad.mullvadvpn.onNodeWithTagAndChildrenText
import net.mullvad.mullvadvpn.viewmodel.CustomDnsItem
import net.mullvad.mullvadvpn.viewmodel.StagedDns
import org.junit.Before
import org.junit.Rule
import org.junit.Test

class VpnSettingsScreenTest {
    @get:Rule val composeTestRule = createComposeRule()

    @Before
    fun setup() {
        MockKAnnotations.init(this)
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDefaultState() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.DefaultUiState(),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_LAST_ITEM_TEST_TAG))

        // Assert
        composeTestRule.apply {
            onNodeWithText("WireGuard MTU").assertExists()
            onNodeWithText("Default").assertExists()
            onNodeWithText("Use custom DNS server").assertExists()
            onNodeWithText("Add a server").assertDoesNotExist()
        }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuCustomValue() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.DefaultUiState(mtu = VALID_DUMMY_MTU_VALUE),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(VALID_DUMMY_MTU_VALUE).assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuClick() {
        // Arrange
        val mockedClickHandler: () -> Unit = mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.DefaultUiState(),
                onMtuCellClick = mockedClickHandler,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Act
        composeTestRule.onNodeWithText("WireGuard MTU").performClick()

        // Assert
        verify { mockedClickHandler.invoke() }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogWithDefaultValue() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.MtuDialogUiState(mtuEditValue = EMPTY_STRING),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(EMPTY_STRING).assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogWithEditValue() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.MtuDialogUiState(mtuEditValue = VALID_DUMMY_MTU_VALUE),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(VALID_DUMMY_MTU_VALUE).assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogTextInput() {
        // Arrange
        val mockedInputHandler: (String) -> Unit = mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.MtuDialogUiState(mtuEditValue = EMPTY_STRING),
                onMtuInputChange = mockedInputHandler,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Act
        composeTestRule.onNodeWithText(EMPTY_STRING).performTextInput(VALID_DUMMY_MTU_VALUE)

        // Assert
        verifyAll { mockedInputHandler.invoke(VALID_DUMMY_MTU_VALUE) }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogSubmitOfValidValue() {
        // Arrange
        val mockedSubmitHandler: () -> Unit = mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.MtuDialogUiState(mtuEditValue = VALID_DUMMY_MTU_VALUE),
                onSaveMtuClick = mockedSubmitHandler,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Act
        composeTestRule.onNodeWithText("Submit").assertIsEnabled().performClick()

        // Assert
        verify { mockedSubmitHandler.invoke() }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogSubmitButtonDisabledWhenInvalidInput() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.MtuDialogUiState(mtuEditValue = INVALID_DUMMY_MTU_VALUE),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Submit").assertIsNotEnabled()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogResetClick() {
        // Arrange
        val mockedClickHandler: () -> Unit = mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.MtuDialogUiState(mtuEditValue = EMPTY_STRING),
                onRestoreMtuClick = mockedClickHandler,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Act
        composeTestRule.onNodeWithText("Reset to default").performClick()

        // Assert
        verify { mockedClickHandler.invoke() }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testMtuDialogCancelClick() {
        // Arrange
        val mockedClickHandler: () -> Unit = mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.MtuDialogUiState(mtuEditValue = EMPTY_STRING),
                onCancelMtuDialogClick = mockedClickHandler,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Cancel").performClick()

        // Assert
        verify { mockedClickHandler.invoke() }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testCustomDnsAddressesAndAddButtonVisibleWhenCustomDnsEnabled() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        isCustomDnsEnabled = true,
                        isAllowLanEnabled = false,
                        customDnsItems =
                            listOf(
                                CustomDnsItem(address = DUMMY_DNS_ADDRESS, false),
                                CustomDnsItem(address = DUMMY_DNS_ADDRESS_2, false),
                                CustomDnsItem(address = DUMMY_DNS_ADDRESS_3, false)
                            )
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_LAST_ITEM_TEST_TAG))
        // Assert
        composeTestRule.apply {
            onNodeWithText(DUMMY_DNS_ADDRESS).assertExists()
            onNodeWithText(DUMMY_DNS_ADDRESS_2).assertExists()
            onNodeWithText(DUMMY_DNS_ADDRESS_3).assertExists()
            onNodeWithText("Add a server").assertExists()
        }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testCustomDnsAddressesAndAddButtonNotVisibleWhenCustomDnsDisabled() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        isCustomDnsEnabled = false,
                        customDnsItems = listOf(CustomDnsItem(address = DUMMY_DNS_ADDRESS, false))
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_LAST_ITEM_TEST_TAG))
        // Assert
        composeTestRule.onNodeWithText(DUMMY_DNS_ADDRESS).assertDoesNotExist()
        composeTestRule.onNodeWithText("Add a server").assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testLanWarningNotShownWhenLanTrafficEnabledAndLocalAddressIsUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        isCustomDnsEnabled = true,
                        isAllowLanEnabled = true,
                        customDnsItems =
                            listOf(CustomDnsItem(address = DUMMY_DNS_ADDRESS, isLocal = true))
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithContentDescription(LOCAL_DNS_SERVER_WARNING).assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testLanWarningNotShowedWhenLanTrafficDisabledAndLocalAddressIsNotUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        isCustomDnsEnabled = true,
                        isAllowLanEnabled = false,
                        customDnsItems =
                            listOf(CustomDnsItem(address = DUMMY_DNS_ADDRESS, isLocal = false))
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithContentDescription(LOCAL_DNS_SERVER_WARNING).assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testLanWarningNotShowedWhenLanTrafficEnabledAndLocalAddressIsNotUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        isCustomDnsEnabled = true,
                        isAllowLanEnabled = true,
                        customDnsItems =
                            listOf(CustomDnsItem(address = DUMMY_DNS_ADDRESS, isLocal = false))
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithContentDescription(LOCAL_DNS_SERVER_WARNING).assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testLanWarningShowedWhenAllowLanEnabledAndLocalDnsAddressIsUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        isCustomDnsEnabled = true,
                        isAllowLanEnabled = false,
                        customDnsItems =
                            listOf(CustomDnsItem(address = DUMMY_DNS_ADDRESS, isLocal = true))
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_LAST_ITEM_TEST_TAG))

        // Assert
        composeTestRule.apply {
            onNodeWithContentDescription(LOCAL_DNS_SERVER_WARNING).assertExists()
        }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testClickAddDns() {
        // Arrange
        val mockedClickHandler: (Int?) -> Unit = mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.DefaultUiState(isCustomDnsEnabled = true),
                onDnsClick = mockedClickHandler,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_LAST_ITEM_TEST_TAG))

        // Act
        composeTestRule.onNodeWithText("Add a server").performClick()

        // Assert
        verify { mockedClickHandler.invoke(null) }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testShowDnsDialogForNewDnsServer() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = false)
                            ),
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Add DNS server").assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testShowDnsDialogForUpdatingDnsServer() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.EditDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = false),
                                index = 0
                            )
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Update DNS server").assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDnsDialogLanWarningShownWhenLanTrafficDisabledAndLocalAddressUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = true),
                                validationResult = StagedDns.ValidationResult.Success
                            ),
                        isAllowLanEnabled = false
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(LOCAL_DNS_SERVER_WARNING).assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDnsDialogLanWarningNotShownWhenLanTrafficEnabledAndLocalAddressUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = true),
                                validationResult = StagedDns.ValidationResult.Success
                            ),
                        isAllowLanEnabled = true
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(LOCAL_DNS_SERVER_WARNING).assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDnsDialogLanWarningNotShownWhenLanTrafficEnabledAndNonLocalAddressUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = false),
                                validationResult = StagedDns.ValidationResult.Success
                            ),
                        isAllowLanEnabled = true
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(LOCAL_DNS_SERVER_WARNING).assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDnsDialogLanWarningNotShownWhenLanTrafficDisabledAndNonLocalAddressUsed() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = false),
                                validationResult = StagedDns.ValidationResult.Success
                            ),
                        isAllowLanEnabled = false
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText(LOCAL_DNS_SERVER_WARNING).assertDoesNotExist()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDnsDialogSubmitButtonDisabledOnInvalidDnsAddress() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = false),
                                validationResult = StagedDns.ValidationResult.InvalidAddress
                            )
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Submit").assertIsNotEnabled()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testDnsDialogSubmitButtonDisabledOnDuplicateDnsAddress() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DnsDialogUiState(
                        stagedDns =
                            StagedDns.NewDns(
                                item = CustomDnsItem(DUMMY_DNS_ADDRESS, isLocal = false),
                                validationResult = StagedDns.ValidationResult.DuplicateAddress
                            )
                    ),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Submit").assertIsNotEnabled()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testShowSelectedTunnelQuantumOption() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(quantumResistant = QuantumResistantState.On),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_QUANTUM_ITEM_OFF_TEST_TAG))

        // Assert
        composeTestRule
            .onNodeWithTagAndChildrenText(testTag = LAZY_LIST_QUANTUM_ITEM_ON_TEST_TAG, text = "On")
            .assertExists()
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testSelectTunnelQuantumOption() {
        // Arrange
        val mockSelectQuantumResistantSettingListener: (QuantumResistantState) -> Unit =
            mockk(relaxed = true)
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState =
                    VpnSettingsUiState.DefaultUiState(
                        quantumResistant = QuantumResistantState.Auto,
                    ),
                onSelectQuantumResistanceSetting = mockSelectQuantumResistantSettingListener,
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }
        composeTestRule
            .onNodeWithTag(LAZY_LIST_TEST_TAG)
            .performScrollToNode(hasTestTag(LAZY_LIST_QUANTUM_ITEM_OFF_TEST_TAG))

        // Assert
        composeTestRule
            .onNodeWithTagAndChildrenText(testTag = LAZY_LIST_QUANTUM_ITEM_ON_TEST_TAG, text = "On")
            .performClick()
        verify(exactly = 1) {
            mockSelectQuantumResistantSettingListener.invoke(QuantumResistantState.On)
        }
    }

    @Test
    @OptIn(ExperimentalMaterialApi::class)
    fun testShowTunnelQuantumInfo() {
        // Arrange
        composeTestRule.setContent {
            VpnSettingsScreen(
                uiState = VpnSettingsUiState.QuantumResistanceInfoDialogUiState(),
                toastMessagesSharedFlow = MutableSharedFlow<String>().asSharedFlow()
            )
        }

        // Assert
        composeTestRule.onNodeWithText("Got it!").assertExists()
    }

    companion object {
        private const val LOCAL_DNS_SERVER_WARNING =
            "The local DNS server will not work unless you enable " +
                "\"Local Network Sharing\" under Preferences."
        private const val EMPTY_STRING = ""
        private const val VALID_DUMMY_MTU_VALUE = "1337"
        private const val INVALID_DUMMY_MTU_VALUE = "1111"
        private const val DUMMY_DNS_ADDRESS = "0.0.0.1"
        private const val DUMMY_DNS_ADDRESS_2 = "0.0.0.2"
        private const val DUMMY_DNS_ADDRESS_3 = "0.0.0.3"
    }
}
