package org.electronicpatientrecord.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge

/** The app's one activity: a thin shell around [HomeScreen] (spec/mobile-apps/index.md §2). NOT FOR CLINICAL USE. */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { HomeScreen() }
    }
}
