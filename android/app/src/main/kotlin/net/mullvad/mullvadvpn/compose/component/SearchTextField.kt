package net.mullvad.mullvadvpn.compose.component

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.ExperimentalMaterialApi
import androidx.compose.material.TextFieldDefaults.TextFieldDecorationBox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import net.mullvad.mullvadvpn.R
import net.mullvad.mullvadvpn.compose.theme.AppTheme
import net.mullvad.mullvadvpn.compose.theme.MullvadWhite10

@Preview
@Composable
private fun PreviewSearchTextField() {
    AppTheme {
        Column(modifier = Modifier.background(color = MaterialTheme.colorScheme.background)) {
            SearchTextField(placeHolder = "Search for...") {}
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchTextField(
    modifier: Modifier = Modifier,
    placeHolder: String = stringResource(id = R.string.search_placeholder),
    backgroundColor: Color = MullvadWhite10,
    enabled: Boolean = true,
    singleLine: Boolean = true,
    interactionSource: MutableInteractionSource = remember { MutableInteractionSource() },
    visualTransformation: VisualTransformation = VisualTransformation.None,
    onValueChange: (String) -> Unit
) {
    var value by rememberSaveable { mutableStateOf("") }

    BasicTextField(
        value = value,
        textStyle =
            MaterialTheme.typography.labelLarge.copy(color = MaterialTheme.colorScheme.onSecondary),
        onValueChange = { text: String ->
            value = text
            onValueChange.invoke(text)
        },
        singleLine = singleLine,
        cursorBrush = SolidColor(MaterialTheme.colorScheme.onSecondary),
        decorationBox =
            @Composable { innerTextField ->
                TextFieldDefaults.TextFieldDecorationBox(
                    value = value,
                    innerTextField = innerTextField,
                    enabled = enabled,
                    singleLine = singleLine,
                    interactionSource = interactionSource,
                    visualTransformation = visualTransformation,
                    leadingIcon = {
                        Image(
                            painter = painterResource(id = R.drawable.icons_search),
                            contentDescription = null,
                            modifier = Modifier.size(width = 24.dp, height = 24.dp),
                            colorFilter =
                                ColorFilter.tint(color = MaterialTheme.colorScheme.onSecondary)
                        )
                    },
                    placeholder = {
                        Text(text = placeHolder, style = MaterialTheme.typography.labelLarge)
                    },
                    shape = MaterialTheme.shapes.medium,
                    colors =
                        TextFieldDefaults.textFieldColors(
                            textColor = MaterialTheme.colorScheme.onSecondary,
                            containerColor = backgroundColor,
                            focusedIndicatorColor = Color.Transparent,
                            unfocusedIndicatorColor = Color.Transparent,
                            cursorColor = MaterialTheme.colorScheme.onSecondary,
                            placeholderColor = MaterialTheme.colorScheme.onSecondary
                        ),
                    contentPadding = PaddingValues(),
                )
            },
        modifier = modifier
    )
}
