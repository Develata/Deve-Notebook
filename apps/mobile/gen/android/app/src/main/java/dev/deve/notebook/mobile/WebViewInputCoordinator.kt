package dev.deve.notebook.mobile

import android.util.Log
import android.view.MotionEvent
import android.view.ViewConfiguration
import android.webkit.WebView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

/** Current-generation Android View focus owner for WebView input connections. */
internal class WebViewInputCoordinator(private val activity: MainActivity) {
  private companion object {
    const val LOG_TAG = "DeveMobile"
  }

  private var webView: WebView? = null
  private var webViewGeneration = 0L
  private var tapCandidate: TapCandidate? = null
  private var touchSlop = 0f

  private data class TapCandidate(
    val source: WebView,
    val generation: Long,
    val pointerId: Int,
    val downRawX: Float,
    val downRawY: Float,
    val downTime: Long,
  )

  fun attach(webView: WebView) {
    detachSource()
    webViewGeneration += 1
    tapCandidate = null
    this.webView = webView
    touchSlop = ViewConfiguration.get(webView.context).scaledTouchSlop.toFloat()
    webView.setOnTouchListener { view, event ->
      if (view === this.webView) onWebViewTouchEvent(event)
      false
    }
    restoreNativeViewFocus()
  }

  fun onWindowFocusChanged(hasFocus: Boolean) {
    if (hasFocus) restoreNativeViewFocus()
  }

  fun detach() {
    detachSource()
    webViewGeneration += 1
    tapCandidate = null
    touchSlop = 0f
    webView = null
  }

  private fun detachSource() {
    webView?.setOnTouchListener(null)
  }

  private fun onWebViewTouchEvent(event: MotionEvent) {
    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN -> beginTapCandidate(event)
      MotionEvent.ACTION_MOVE -> retainTapCandidateIfStationary(event)
      MotionEvent.ACTION_UP -> completeTapCandidate(event)
      MotionEvent.ACTION_POINTER_DOWN,
      MotionEvent.ACTION_POINTER_UP,
      MotionEvent.ACTION_CANCEL,
      MotionEvent.ACTION_OUTSIDE -> tapCandidate = null
    }
  }

  private fun beginTapCandidate(event: MotionEvent) {
    tapCandidate = null
    if (event.pointerCount != 1) return
    val source = webView ?: return
    val generation = webViewGeneration
    if (!activity.hasWindowFocus() || !source.isAttachedToWindow || !contains(source, event.rawX, event.rawY)) return
    tapCandidate = TapCandidate(
      source = source,
      generation = generation,
      pointerId = event.getPointerId(0),
      downRawX = event.rawX,
      downRawY = event.rawY,
      downTime = event.eventTime,
    )
  }

  private fun retainTapCandidateIfStationary(event: MotionEvent) {
    val candidate = tapCandidate ?: return
    if (event.pointerCount != 1 || event.findPointerIndex(candidate.pointerId) < 0) {
      tapCandidate = null
      return
    }
    val dx = event.rawX - candidate.downRawX
    val dy = event.rawY - candidate.downRawY
    if ((dx * dx) + (dy * dy) > touchSlop * touchSlop) tapCandidate = null
  }

  private fun completeTapCandidate(event: MotionEvent) {
    val candidate = tapCandidate
    tapCandidate = null
    if (candidate == null || event.pointerCount != 1) return
    if (event.getPointerId(0) != candidate.pointerId) return
    if (webView !== candidate.source || webViewGeneration != candidate.generation) return
    val dx = event.rawX - candidate.downRawX
    val dy = event.rawY - candidate.downRawY
    if ((dx * dx) + (dy * dy) > touchSlop * touchSlop) return
    if (event.eventTime - candidate.downTime >= ViewConfiguration.getLongPressTimeout()) return
    if (!contains(candidate.source, event.rawX, event.rawY)) return
    probeActiveEditorAndShowIme(candidate.source, candidate.generation, event.rawX, event.rawY)
  }

  private fun probeActiveEditorAndShowIme(
    source: WebView,
    generation: Long,
    rawX: Float,
    rawY: Float,
  ) {
    val density = activity.resources.displayMetrics.density
    if (!density.isFinite() || density <= 0f) return
    val location = IntArray(2)
    source.getLocationOnScreen(location)
    val localX = rawX - location[0]
    val localY = rawY - location[1]
    val xCss = localX / density
    val yCss = localY / density
    val script = """
      (() => {
        const target = document.elementFromPoint($xCss, $yCss);
        const editor = target?.closest?.('.cm-content[contenteditable="true"]') ?? null;
        return editor !== null && document.activeElement === editor;
      })()
    """.trimIndent()
    try {
      source.evaluateJavascript(script) { accepted ->
        activity.runOnUiThread {
          if (webView !== source || webViewGeneration != generation || accepted != "true") return@runOnUiThread
          if (!activity.hasWindowFocus() || !source.isAttachedToWindow || !source.isShown) return@runOnUiThread
          if (!source.hasFocus() && !source.requestFocus()) {
            Log.w(LOG_TAG, "deve_mobile input checkpoint: android_webview_input_focus_unavailable")
            return@runOnUiThread
          }
          try {
            WindowCompat.getInsetsController(activity.window, source)
              .show(WindowInsetsCompat.Type.ime())
          } catch (_error: RuntimeException) {
            Log.w(LOG_TAG, "deve_mobile input checkpoint: android_webview_ime_show_failed")
          }
        }
      }
    } catch (_error: RuntimeException) {
      Log.w(LOG_TAG, "deve_mobile input checkpoint: android_webview_input_probe_failed")
    }
  }

  private fun contains(source: WebView, rawX: Float, rawY: Float): Boolean {
    if (source.width <= 0 || source.height <= 0 || !source.isShown) return false
    val location = IntArray(2)
    source.getLocationOnScreen(location)
    val localX = rawX - location[0]
    val localY = rawY - location[1]
    return localX >= 0f && localY >= 0f && localX < source.width && localY < source.height
  }

  private fun restoreNativeViewFocus() {
    val source = webView ?: return
    source.isFocusable = true
    source.isFocusableInTouchMode = true
    if (!source.hasFocus() && !source.requestFocus()) {
      Log.w(LOG_TAG, "deve_mobile input checkpoint: android_webview_input_focus_unavailable")
    }
  }
}
