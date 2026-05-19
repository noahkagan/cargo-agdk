package org.cargoagdk.stocksample;

import com.google.androidgamesdk.GameActivity;

// Minimal AGDK GameActivity host. The stock sample exists only to
// prime the Gradle dependency cache during `cargo agdk publish`; the
// JNI side never ships in real consumers' bundles, so a stub
// extending GameActivity is enough.
public class MainActivity extends GameActivity {
}
