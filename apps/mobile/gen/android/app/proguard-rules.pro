# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# Rust calls these platform-owned MainActivity methods by exact JNI name.
-keepclassmembers class dev.deve.notebook.mobile.MainActivity {
    public boolean installNativeSessionCookie(long, android.webkit.WebView, java.lang.String, java.lang.String, java.lang.String);
    private native void nativeSessionCookieInstallCompleted(long, int);
    public boolean installUseLocalBackendControl();
    public boolean resetUseLocalBackendControl();
    public boolean removeUseLocalBackendControl();
    public boolean scheduleBackendRecoveryColdStart();
}

# Tao/Wry resolves the Kotlin `id` property getter by its exact JNI name.
# R8 cannot see that Rust-side lookup and otherwise removes getId() in release builds.
-keepclassmembers class dev.deve.notebook.mobile.WryActivity {
    public int getId();
}

-keepclassmembers class dev.deve.notebook.mobile.RecoveryAnchorActivity {
    public boolean scheduleBackendRecoveryColdStart();
}
