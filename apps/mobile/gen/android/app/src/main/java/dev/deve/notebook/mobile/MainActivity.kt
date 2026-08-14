package dev.deve.notebook.mobile

import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.os.Looper
import android.util.Log
import android.webkit.CookieManager
import android.webkit.WebView
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  private companion object {
    const val NATIVE_COOKIE_RETAINED = 1
    const val NATIVE_COOKIE_REJECTED = 2
    const val NATIVE_COOKIE_NOT_RETAINED = 3
    const val NATIVE_COOKIE_VERIFICATION_FAILED = 4
    const val NATIVE_COOKIE_SETUP_FAILED = 5
    const val NATIVE_COOKIE_LOG_TAG = "DeveMobile"
  }

  private val useLocalBackendControl = UseLocalBackendControl(this) { requestUseLocalBackend() }
  private val uiBackDispatcher = UiBackDispatcher(this)
  private val nativePresentationDispatcher = NativePresentationDispatcher(this)
  private val webViewInputCoordinator = WebViewInputCoordinator(this)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT),
      navigationBarStyle = SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT),
    )
    super.onCreate(savedInstanceState)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      window.isNavigationBarContrastEnforced = false
    }
    uiBackDispatcher.install()
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    uiBackDispatcher.attach(webView)
    nativePresentationDispatcher.attach(webView)
    webViewInputCoordinator.attach(webView)
    useLocalBackendControl.attach(webView)
  }

  override fun onResume() {
    super.onResume()
    useLocalBackendControl.restoreIfDesired()
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    nativePresentationDispatcher.onWindowFocusChanged(hasFocus)
    webViewInputCoordinator.onWindowFocusChanged(hasFocus)
  }

  override fun onDestroy() {
    useLocalBackendControl.detach()
    nativePresentationDispatcher.detach()
    webViewInputCoordinator.detach()
    uiBackDispatcher.detach()
    super.onDestroy()
  }

  fun installUseLocalBackendControl(): Boolean {
    return useLocalBackendControl.install()
  }

  fun resetUseLocalBackendControl(): Boolean = installUseLocalBackendControl()

  fun removeUseLocalBackendControl(): Boolean {
    return useLocalBackendControl.remove()
  }

  fun scheduleBackendRecoveryColdStart(): Boolean = scheduleDeveColdStart()

  fun installNativeSessionCookie(
    requestId: Long,
    webView: WebView,
    installUrl: String,
    verificationUrl: String,
    setCookie: String,
  ): Boolean {
    return try {
      if (Looper.myLooper() == Looper.getMainLooper()) {
        beginNativeSessionCookieInstall(
          requestId,
          webView,
          installUrl,
          verificationUrl,
          setCookie,
        )
      } else {
        runOnUiThread {
          if (!beginNativeSessionCookieInstall(
              requestId,
              webView,
              installUrl,
              verificationUrl,
              setCookie,
            )) {
            completeNativeSessionCookieInstall(requestId, NATIVE_COOKIE_SETUP_FAILED)
          }
        }
        true
      }
    } catch (_error: RuntimeException) {
      false
    }
  }

  private fun beginNativeSessionCookieInstall(
    requestId: Long,
    webView: WebView,
    installUrl: String,
    verificationUrl: String,
    setCookie: String,
  ): Boolean {
    return try {
      val manager = CookieManager.getInstance()
      manager.setAcceptCookie(true)
      manager.setAcceptThirdPartyCookies(webView, true)
      val expectedPair = setCookie.substringBefore(';').trim()
      manager.setCookie(installUrl, setCookie) { accepted ->
        val completion = if (accepted != true) {
          NATIVE_COOKIE_REJECTED
        } else {
          try {
            val retained = manager.getCookie(verificationUrl)
              ?.split(';')
              ?.map { it.trim() }
              ?.any { it == expectedPair } == true
            if (retained) {
              manager.flush()
              NATIVE_COOKIE_RETAINED
            } else {
              NATIVE_COOKIE_NOT_RETAINED
            }
          } catch (_error: RuntimeException) {
            NATIVE_COOKIE_VERIFICATION_FAILED
          }
        }
        completeNativeSessionCookieInstall(requestId, completion)
      }
      true
    } catch (_error: RuntimeException) {
      false
    }
  }

  private fun completeNativeSessionCookieInstall(requestId: Long, completion: Int) {
    val category = when (completion) {
      NATIVE_COOKIE_RETAINED -> "android_native_cookie_retained"
      NATIVE_COOKIE_REJECTED -> "android_native_cookie_callback_rejected"
      NATIVE_COOKIE_NOT_RETAINED -> "android_native_cookie_not_retained"
      NATIVE_COOKIE_VERIFICATION_FAILED -> "android_native_cookie_verification_failed"
      NATIVE_COOKIE_SETUP_FAILED -> "android_native_cookie_jni_setup_failed"
      else -> "android_native_cookie_callback_invalid"
    }
    if (completion == NATIVE_COOKIE_RETAINED) {
      Log.i(
        NATIVE_COOKIE_LOG_TAG,
        "deve_mobile native session cookie checkpoint: $category",
      )
    } else {
      Log.e(
        NATIVE_COOKIE_LOG_TAG,
        "deve_mobile native session cookie handoff failed closed: $category",
      )
    }
    nativeSessionCookieInstallCompleted(requestId, completion)
  }

  private external fun requestUseLocalBackend(): Boolean
  private external fun nativeSessionCookieInstallCompleted(requestId: Long, completion: Int)
}
