package dev.deve.notebook.mobile

import android.os.Bundle
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.ViewGroup
import android.webkit.CookieManager
import android.webkit.WebView
import android.widget.Button
import android.widget.FrameLayout
import androidx.activity.enableEdgeToEdge
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class MainActivity : TauriActivity() {
  private companion object {
    const val NATIVE_COOKIE_RETAINED = 1
    const val NATIVE_COOKIE_REJECTED = 2
    const val NATIVE_COOKIE_NOT_RETAINED = 3
    const val NATIVE_COOKIE_VERIFICATION_FAILED = 4
    const val NATIVE_COOKIE_SETUP_FAILED = 5
    const val NATIVE_COOKIE_LOG_TAG = "DeveMobile"
  }

  private var useLocalBackendButton: Button? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  fun installUseLocalBackendControl(): Boolean {
    return onUiThreadConfirmed { installUseLocalBackendControlOnMainThread() }
  }

  private fun installUseLocalBackendControlOnMainThread(): Boolean {
    val existing = useLocalBackendButton
    if (existing != null) {
      existing.isEnabled = true
      existing.text = "Use Local Backend"
      return true
    }
    val root = findViewById<ViewGroup>(android.R.id.content) ?: return false
    val button = Button(this).apply {
      text = "Use Local Backend"
      contentDescription = "Use Local Backend"
      isAllCaps = false
      setOnClickListener {
        if (requestUseLocalBackend()) {
          isEnabled = false
          text = "Switching…"
        }
      }
    }
    val margin = (12 * resources.displayMetrics.density).toInt()
    val topMargin = (48 * resources.displayMetrics.density).toInt()
    val height = (48 * resources.displayMetrics.density).toInt()
    root.addView(
      button,
      FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.WRAP_CONTENT,
        height,
        Gravity.TOP or Gravity.END,
      ).apply {
        marginEnd = margin
        this.topMargin = topMargin
      },
    )
    useLocalBackendButton = button
    return true
  }

  fun resetUseLocalBackendControl(): Boolean = installUseLocalBackendControl()

  fun removeUseLocalBackendControl(): Boolean {
    return onUiThreadConfirmed {
      val button = useLocalBackendButton ?: return@onUiThreadConfirmed true
      (button.parent as? ViewGroup)?.removeView(button) ?: return@onUiThreadConfirmed false
      useLocalBackendButton = null
      true
    }
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
      NATIVE_COOKIE_RETAINED -> null
      NATIVE_COOKIE_REJECTED -> "android_native_cookie_callback_rejected"
      NATIVE_COOKIE_NOT_RETAINED -> "android_native_cookie_not_retained"
      NATIVE_COOKIE_VERIFICATION_FAILED -> "android_native_cookie_verification_failed"
      NATIVE_COOKIE_SETUP_FAILED -> "android_native_cookie_jni_setup_failed"
      else -> "android_native_cookie_callback_invalid"
    }
    if (category != null) {
      Log.e(
        NATIVE_COOKIE_LOG_TAG,
        "deve_mobile native session cookie handoff failed closed: $category",
      )
    }
    nativeSessionCookieInstallCompleted(requestId, completion)
  }

  private fun onUiThreadConfirmed(action: () -> Boolean): Boolean {
    if (Looper.myLooper() == Looper.getMainLooper()) return action()
    val latch = CountDownLatch(1)
    var result = false
    runOnUiThread {
      result = action()
      latch.countDown()
    }
    return latch.await(2, TimeUnit.SECONDS) && result
  }

  private external fun requestUseLocalBackend(): Boolean
  private external fun nativeSessionCookieInstallCompleted(requestId: Long, completion: Int)
}
