package dev.deve.notebook.mobile

import android.util.Log
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.ScriptHandler
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature

/** Generation-bound, authority-neutral Android presentation hints for the Web shell. */
internal class NativePresentationDispatcher(private val activity: MainActivity) {
  private companion object {
    const val LOG_TAG = "DeveMobile"
    const val PRESENTATION_EVENT = "deve-native-presentation-change"
    const val DOCUMENT_BRIDGE = "devePresentationDocument"
    const val DOCUMENT_MESSAGE = "document-start"
    const val DOCUMENT_START_SCRIPT = """
      Object.defineProperty(globalThis, "__DEVE_ANDROID_PRESENTATION_PENDING__", {
        value: true,
        configurable: false,
        enumerable: false,
        writable: false
      });
      globalThis.devePresentationDocument.postMessage("document-start");
    """
    val RETRY_DELAYS_MS = longArrayOf(0L, 100L, 500L, 1_000L, 2_000L, 5_000L)
    val ALLOWED_ORIGINS = setOf("*")
  }

  private var webView: WebView? = null
  private var webViewGeneration = 0L
  private var publishEpoch = 0L
  private var pendingPublish: Runnable? = null
  private var layoutListener: View.OnLayoutChangeListener? = null
  private var insetsObserver: View? = null
  private var insetsObserverRoot: ViewGroup? = null
  private var documentScriptHandler: ScriptHandler? = null
  private var documentBridgeInstalled = false
  private var readyLoggedGeneration: Long? = null
  private var unavailableLoggedEpoch: Long? = null

  fun attach(webView: WebView) {
    detachSource()
    webViewGeneration += 1
    this.webView = webView
    installDocumentLifecycleBridge(webView)
    val listener = View.OnLayoutChangeListener { _, left, _, right, _, oldLeft, _, oldRight, _ ->
      if (left != oldLeft || right != oldRight) beginPublish()
    }
    layoutListener = listener
    webView.addOnLayoutChangeListener(listener)
    installInsetsObserver(webView)
    beginPublish()
  }

  fun onWindowFocusChanged(hasFocus: Boolean) {
    if (hasFocus) beginPublish()
  }

  fun detach() {
    detachSource()
    webView = null
  }

  private fun detachSource() {
    publishEpoch += 1
    val source = webView
    pendingPublish?.let { source?.removeCallbacks(it) }
    pendingPublish = null
    layoutListener?.let { source?.removeOnLayoutChangeListener(it) }
    layoutListener = null
    insetsObserver?.let { observer ->
      ViewCompat.setOnApplyWindowInsetsListener(observer, null)
      insetsObserverRoot?.removeView(observer)
    }
    insetsObserver = null
    insetsObserverRoot = null
    try {
      documentScriptHandler?.remove()
    } catch (_error: RuntimeException) {
      Log.w(LOG_TAG, "deve_mobile presentation checkpoint: android_presentation_document_script_remove_failed")
    }
    documentScriptHandler = null
    if (documentBridgeInstalled && source != null) {
      try {
        WebViewCompat.removeWebMessageListener(source, DOCUMENT_BRIDGE)
      } catch (_error: RuntimeException) {
        Log.w(LOG_TAG, "deve_mobile presentation checkpoint: android_presentation_document_bridge_remove_failed")
      }
    }
    documentBridgeInstalled = false
  }

  private fun installDocumentLifecycleBridge(source: WebView) {
    val supported = WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER) &&
      WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
    if (!supported) {
      Log.w(
        LOG_TAG,
        "deve_mobile presentation checkpoint: android_presentation_document_bridge_unavailable",
      )
      return
    }
    try {
      WebViewCompat.addWebMessageListener(
        source,
        DOCUMENT_BRIDGE,
        ALLOWED_ORIGINS,
      ) { view, message, _, isMainFrame, _ ->
        if (isMainFrame && view === webView && message.data == DOCUMENT_MESSAGE) beginPublish()
      }
      documentBridgeInstalled = true
      documentScriptHandler = WebViewCompat.addDocumentStartJavaScript(
        source,
        DOCUMENT_START_SCRIPT.trimIndent(),
        ALLOWED_ORIGINS,
      )
    } catch (_error: RuntimeException) {
      if (documentBridgeInstalled) {
        try {
          WebViewCompat.removeWebMessageListener(source, DOCUMENT_BRIDGE)
        } catch (_removeError: RuntimeException) {
          Log.w(LOG_TAG, "deve_mobile presentation checkpoint: android_presentation_document_bridge_remove_failed")
        }
      }
      documentBridgeInstalled = false
      documentScriptHandler = null
      Log.w(
        LOG_TAG,
        "deve_mobile presentation checkpoint: android_presentation_document_bridge_unavailable",
      )
    }
  }

  private fun beginPublish() {
    val source = webView ?: return
    publishEpoch += 1
    val epoch = publishEpoch
    unavailableLoggedEpoch = null
    pendingPublish?.let(source::removeCallbacks)
    pendingPublish = null
    scheduleInvalidation(source, webViewGeneration, epoch, 0)
  }

  private fun scheduleInvalidation(source: WebView, generation: Long, epoch: Long, attempt: Int) {
    if (!isCurrent(source, generation, epoch)) return
    if (attempt >= RETRY_DELAYS_MS.size) {
      publishUnavailable(epoch)
      return
    }
    val runnable = Runnable {
      if (!isCurrent(source, generation, epoch)) return@Runnable
      pendingPublish = null
      try {
        source.evaluateJavascript(pendingScript(generation, epoch)) { rawAck ->
          activity.runOnUiThread {
            if (!isCurrent(source, generation, epoch)) return@runOnUiThread
            if (rawAck == "true") schedulePublish(source, generation, epoch, 0)
            else scheduleInvalidation(source, generation, epoch, attempt + 1)
          }
        }
      } catch (_error: RuntimeException) {
        scheduleInvalidation(source, generation, epoch, attempt + 1)
      }
    }
    pendingPublish = runnable
    source.postDelayed(runnable, RETRY_DELAYS_MS[attempt])
  }

  private fun schedulePublish(source: WebView, generation: Long, epoch: Long, attempt: Int) {
    if (!isCurrent(source, generation, epoch)) return
    if (attempt >= RETRY_DELAYS_MS.size) {
      publishUnavailable(epoch)
      return
    }
    val runnable = Runnable {
      if (!isCurrent(source, generation, epoch)) return@Runnable
      pendingPublish = null
      val widthPx = source.width
      val density = activity.resources.displayMetrics.density.toDouble()
      val rootInsets = ViewCompat.getRootWindowInsets(activity.window.decorView)
      val gestures = rootInsets?.getInsets(WindowInsetsCompat.Type.systemGestures())
      if (widthPx <= 0 || !density.isFinite() || density <= 0.0 || gestures == null) {
        schedulePublish(source, generation, epoch, attempt + 1)
        return@Runnable
      }
      val script = presentationScript(
        generation = generation,
        epoch = epoch,
        widthPx = widthPx,
        leftPx = gestures.left,
        rightPx = gestures.right,
        density = density,
      )
      try {
        source.evaluateJavascript(script) { rawAck ->
          activity.runOnUiThread {
            if (!isCurrent(source, generation, epoch)) return@runOnUiThread
            if (rawAck == "true") {
              unavailableLoggedEpoch = null
              if (readyLoggedGeneration != generation) {
                readyLoggedGeneration = generation
                Log.i(
                  LOG_TAG,
                  "deve_mobile presentation checkpoint: android_system_gesture_insets_ready",
                )
              }
            } else {
              schedulePublish(source, generation, epoch, attempt + 1)
            }
          }
        }
      } catch (_error: RuntimeException) {
        schedulePublish(source, generation, epoch, attempt + 1)
      }
    }
    pendingPublish = runnable
    source.postDelayed(runnable, RETRY_DELAYS_MS[attempt])
  }

  private fun isCurrent(source: WebView, generation: Long, epoch: Long): Boolean =
    webView === source && webViewGeneration == generation && publishEpoch == epoch

  private fun publishUnavailable(epoch: Long) {
    pendingPublish = null
    if (unavailableLoggedEpoch == epoch) return
    unavailableLoggedEpoch = epoch
    Log.w(
      LOG_TAG,
      "deve_mobile presentation checkpoint: android_system_gesture_insets_unavailable",
    )
  }

  private fun installInsetsObserver(source: WebView) {
    val root = activity.findViewById<ViewGroup>(android.R.id.content) ?: return
    val observer = View(activity).apply {
      alpha = 0f
      isClickable = false
      importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    ViewCompat.setOnApplyWindowInsetsListener(observer) { _, insets ->
      if (webView === source) beginPublish()
      insets
    }
    root.addView(observer, FrameLayout.LayoutParams(0, 0))
    insetsObserver = observer
    insetsObserverRoot = root
    ViewCompat.requestApplyInsets(observer)
  }

  private fun pendingScript(generation: Long, epoch: Long): String = """
    (() => {
      delete window.__DEVE_ANDROID_PRESENTATION__;
      const detail = {
        kind: "system-gesture-insets-pending",
        generation: $generation,
        epoch: $epoch,
        listenerSeen: false
      };
      window.dispatchEvent(new CustomEvent("$PRESENTATION_EVENT", { detail }));
      return detail.listenerSeen === true;
    })()
  """.trimIndent()

  private fun presentationScript(
    generation: Long,
    epoch: Long,
    widthPx: Int,
    leftPx: Int,
    rightPx: Int,
    density: Double,
  ): String = """
    (() => {
      const snapshot = Object.freeze({
        kind: "system-gesture-insets",
        generation: $generation,
        epoch: $epoch,
        widthPx: $widthPx,
        leftPx: $leftPx,
        rightPx: $rightPx,
        density: $density
      });
      window.__DEVE_ANDROID_PRESENTATION__ = snapshot;
      const detail = { ...snapshot, listenerSeen: false };
      window.dispatchEvent(new CustomEvent("$PRESENTATION_EVENT", { detail }));
      return detail.listenerSeen === true;
    })()
  """.trimIndent()
}
