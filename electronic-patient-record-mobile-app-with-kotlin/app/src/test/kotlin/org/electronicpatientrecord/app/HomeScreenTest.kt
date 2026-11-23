// The home screen and the banner, read from the semantics tree that TalkBack reads; Robolectric runs them on the JVM.
package org.electronicpatientrecord.app

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import org.electronicpatientrecord.core.BannerText
import org.electronicpatientrecord.core.HumanName
import org.electronicpatientrecord.core.Identifier
import org.electronicpatientrecord.core.Patient
import org.electronicpatientrecord.core.StoredPatient
import org.electronicpatientrecord.core.Strings
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class HomeScreenTest {
    @get:Rule val rule = createComposeRule()

    private val isHeading = SemanticsMatcher.keyIsDefined(SemanticsProperties.Heading)

    @Test fun theNoticeIsOnTheFirstScreenAndSpokenWithItsPrefix() {
        rule.setContent { HomeScreen() }
        rule.onNodeWithText(Strings.ENGLISH.homeTitle).assertIsDisplayed().assert(isHeading)
        rule.onNodeWithContentDescription("Important\nNot for clinical use. Do not enter real patient data.").assertExists()
    }

    @Test fun theSampleBannerIsADrillAndNamedAsAGroup() {
        rule.setContent { HomeScreen() }
        rule.onNodeWithContentDescription(Strings.ENGLISH.bannerLabel).assertExists()
        rule.onNodeWithText(Strings.ENGLISH.bannerDrill).assertExists()
        rule.onNodeWithText("FEATHERSTONEHAUGH-MONTGOMERY, Alexandra Bartholomew").assert(isHeading)
    }

    @Test fun anAbsentPatientSaysSoInWords() {
        rule.setContent { PatientBanner(StoredPatient("p", 1, "a", Patient())) }
        rule.onNodeWithText("not available").assertExists()
        rule.onNodeWithText("Born: Birth date not recorded").assertExists()
        rule.onNodeWithText("National health number: none recorded").assertExists()
    }

    /**
     * At 320 dp wide and twice the font size, every banner line is laid out whole: every character is placed, no line ends
     * in an ellipsis, no line is wider than the banner, and the long name wraps (H-061). `hasVisualOverflow` is not used:
     * for `BasicText` the semantics action's layout result does not carry the size the node was measured at.
     */
    @Test fun aLongBannerWrapsAndNeverTruncatesAtTheLargestText() {
        val stored = StoredPatient(
            "p", 1, "a",
            Patient(
                names = listOf(HumanName("Featherstonehaugh-Montgomery-Wolfeschlegelsteinhausen", listOf("Alexandra", "Bartholomew"))),
                birthDate = "1980-03-14",
                identifiers = listOf(Identifier("urn:example:a-very-long-medical-record-number-system", "MRN-0001-0002-0003-0004")),
            ),
        )
        var widthPx = 0f
        rule.setContent {
            val density = LocalDensity.current
            widthPx = with(density) { 320.dp.toPx() }
            CompositionLocalProvider(LocalDensity provides Density(density.density, fontScale = 2f)) {
                // In a scroll, as on the home screen: the height is unbounded, so only the width can cut a line.
                Column(Modifier.verticalScroll(rememberScrollState())) { PatientBanner(stored, Modifier.width(320.dp)) }
            }
        }
        val banner = BannerText.of(stored)
        for (line in banner.lines) {
            val node = rule.onNodeWithText(line).assertExists().fetchSemanticsNode()
            val layout = layoutOf(node)
            assertEquals(line, layout.layoutInput.text.text)
            assertEquals("not every character placed: $line", line.length, layout.getLineEnd(layout.lineCount - 1))
            assertFalse("ellipsis in: $line", (0 until layout.lineCount).any { layout.isLineEllipsized(it) })
            assertTrue("wider than the banner: $line", node.boundsInRoot.width <= widthPx + 0.5f)
        }
        assertTrue("the long name should wrap at this width", layoutOf(rule.onNodeWithText(banner.name).fetchSemanticsNode()).lineCount > 1)
    }

    private fun layoutOf(node: SemanticsNode): TextLayoutResult {
        val layouts = mutableListOf<TextLayoutResult>()
        node.config.getOrNull(SemanticsActions.GetTextLayoutResult)?.action?.invoke(layouts)
        return layouts.single()
    }

    @Test fun theBannerReadsInOrderAlertsFirst() {
        rule.setContent { PatientBanner(SyntheticPatients.SAMPLE) }
        val lines = BannerText.of(SyntheticPatients.SAMPLE).lines
        val tops = lines.map { rule.onNode(hasText(it)).fetchSemanticsNode().boundsInRoot.top }
        assertEquals(tops.sorted(), tops)
    }
}
