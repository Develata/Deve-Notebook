package dev.deve.notebook.mobile

import android.util.Log
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONObject
import org.json.JSONTokener

/** Presentation-only Android Back adapter for the Web UiBackCoordinator. */
internal class UiBackDispatcher(private val activity: MainActivity) {
  private companion object {
    const val LOG_TAG = "DeveMobile"
    const val REQUEST_EVENT = "deve-native-back-request"
    const val ACK_TIMEOUT_MS = 750L
  }

  private var webView: WebView? = null
  private var nextRequestId = 1L
  private var webViewGeneration = 0L
  private var activeRequestId: Long? = null
  private var activeRequestGeneration: Long? = null
  private var activeRequestSource: WebView? = null
  private var activeRequestTimeout: Runnable? = null

  fun install() {
    activity.onBackPressedDispatcher.addCallback(activity, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        if (!dismissVisibleIme()) requestUiBack()
      }
    })
  }

  fun attach(webView: WebView) {
    retireActiveRequest()
    webViewGeneration += 1
    this.webView = webView
  }

  fun detach() {
    retireActiveRequest()
    webView = null
  }

  private fun dismissVisibleIme(): Boolean {
    val decorView = activity.window.decorView
    val insets = ViewCompat.getRootWindowInsets(decorView) ?: run {
      Log.w(
        LOG_TAG,
        "deve_mobile ui back checkpoint: android_ui_back_ime_visibility_unavailable",
      )
      return true
    }
    if (!insets.isVisible(WindowInsetsCompat.Type.ime())) return false
    val controller = WindowCompat.getInsetsController(activity.window, decorView)
    try {
      controller.hide(WindowInsetsCompat.Type.ime())
      Log.i(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_ime_dismissed")
    } catch (_error: RuntimeException) {
      Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_ime_dismiss_failed")
    }
    return true
  }

  private fun retireActiveRequest() {
    val source = activeRequestSource
    val timeout = activeRequestTimeout
    if (source != null && timeout != null) source.removeCallbacks(timeout)
    activeRequestId = null
    activeRequestGeneration = null
    activeRequestSource = null
    activeRequestTimeout = null
  }

  private fun requestIsCurrent(requestId: Long, generation: Long, source: WebView): Boolean =
    activeRequestId == requestId &&
      activeRequestGeneration == generation &&
      activeRequestSource === source &&
      webViewGeneration == generation &&
      webView === source

  private fun requestUiBack() {
    val webView = webView ?: run {
      Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_webview_unavailable")
      return
    }
    if (activeRequestId != null) {
      Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_request_in_flight")
      return
    }

    val requestId = nextRequestId++
    val generation = webViewGeneration
    activeRequestId = requestId
    activeRequestGeneration = generation
    activeRequestSource = webView
    val timeout = Runnable {
      if (requestIsCurrent(requestId, generation, webView)) {
        retireActiveRequest()
        Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_ack_timeout")
      }
    }
    activeRequestTimeout = timeout
    webView.postDelayed(timeout, ACK_TIMEOUT_MS)
    val script = """
      (() => {
        const detail = { requestId: "$requestId", outcome: null, listenerSeen: false };
        window.dispatchEvent(new CustomEvent("$REQUEST_EVENT", { detail }));
        return detail;
      })()
    """.trimIndent()
    webView.evaluateJavascript(script) { rawAck ->
      activity.runOnUiThread {
        if (!requestIsCurrent(requestId, generation, webView)) return@runOnUiThread
        retireActiveRequest()
        val ack = parseBackAck(rawAck) ?: run {
          Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_ack_invalid")
          return@runOnUiThread
        }
        if (ack.optString("requestId") != requestId.toString()) {
          Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_ack_stale")
          return@runOnUiThread
        }
        when (ack.optString("outcome")) {
          "Unhandled" -> {
            if (activity.moveTaskToBack(true)) {
              Log.i(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_root_backgrounded")
            } else {
              Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_background_failed")
            }
          }
          "Handled" -> Log.i(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_handled")
          else -> if (!ack.optBoolean("listenerSeen", false)) {
            Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_listener_missing")
          } else {
            Log.w(LOG_TAG, "deve_mobile ui back checkpoint: android_ui_back_outcome_invalid")
          }
        }
      }
    }
  }

  private fun parseBackAck(rawAck: String?): JSONObject? {
    if (rawAck.isNullOrBlank() || rawAck == "null") return null
    return try {
      when (val decoded = JSONTokener(rawAck).nextValue()) {
        is JSONObject -> decoded
        is String -> JSONTokener(decoded).nextValue() as? JSONObject
        else -> null
      }
    } catch (_error: Exception) {
      null
    }
  }
}
