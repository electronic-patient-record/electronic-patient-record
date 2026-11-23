package org.electronicpatientrecord.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.lilydesignsystem.compose.LilyCard
import io.github.lilydesignsystem.compose.LocalLilyTextStyles
import io.github.lilydesignsystem.compose.WarningText
import org.electronicpatientrecord.core.Strings

/**
 * The first screen: the product name, the not-for-clinical-use notice and a sample banner on a synthetic patient.
 * Nothing here calls the service; sign-in and search are MP-5 and MP-7.
 */
@Composable
fun HomeScreen(strings: Strings = Strings.ENGLISH) {
    val styles = LocalLilyTextStyles.current
    Column(
        modifier = Modifier
            .fillMaxSize()
            .safeDrawingPadding()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        BasicText(
            strings.homeTitle,
            modifier = Modifier.semantics { heading() },
            style = styles.heading.copy(fontSize = 30.sp),
        )
        WarningText(prefix = strings.warningPrefix, text = strings.notClinical)
        LilyCard(heading = strings.sampleHeading) {
            PatientBanner(SyntheticPatients.SAMPLE, strings = strings)
        }
    }
}
