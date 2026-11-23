package org.electronicpatientrecord.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.isTraversalGroup
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.lilydesignsystem.compose.LocalLilyTextStyles
import org.electronicpatientrecord.core.BannerText
import org.electronicpatientrecord.core.StoredPatient
import org.electronicpatientrecord.core.Strings


/**
 * The patient banner (spec/mobile-apps/index.md §3 item 2; hazard H-061).
 *
 * Every line wraps and none is ever shortened: `BasicText` with its defaults (soft wrap, no line limit), and no
 * `maxLines` or ellipsis anywhere. TalkBack reads it as one group named by [Strings.bannerLabel], alerts first.
 */
@Composable
fun PatientBanner(stored: StoredPatient, modifier: Modifier = Modifier, strings: Strings = Strings.ENGLISH) {
    val banner = BannerText.of(stored, strings)
    val styles = LocalLilyTextStyles.current
    Column(
        modifier = modifier
            .fillMaxWidth()
            .semantics {
                isTraversalGroup = true
                contentDescription = strings.bannerLabel
            },
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        for (alert in banner.alerts) BasicText(alert, style = styles.strong)
        BasicText(
            banner.name,
            modifier = Modifier.semantics { heading() },
            style = styles.heading.copy(fontWeight = FontWeight.Bold, fontSize = 22.sp),
        )
        BasicText(banner.born, style = styles.body)
        BasicText(banner.nationalNumber, style = styles.body)
        BasicText(banner.otherIdentifier, style = styles.body)
    }
}
