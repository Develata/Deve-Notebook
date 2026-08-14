package dev.deve.notebook.mobile

import android.os.Looper
import android.view.Gravity
import android.view.ViewGroup
import android.webkit.WebView
import android.widget.Button
import android.widget.FrameLayout
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class UseLocalBackendControl(
  private val activity: MainActivity,
  private val requestLocalBackend: () -> Boolean,
) {
  private companion object {
    @Volatile
    var desired = false
  }

  private var button: Button? = null
  private var webView: WebView? = null
  private var generation = 0L

  fun attach(source: WebView) {
    generation += 1
    webView = source
    scheduleRestore(source, generation)
  }

  fun detach() {
    generation += 1
    webView = null
    button = null
  }

  fun install(): Boolean {
    desired = true
    return onUiThreadConfirmed { installOnMainThread() }
  }

  fun restoreIfDesired() {
    if (!desired) return
    if (Looper.myLooper() == Looper.getMainLooper()) {
      scheduleCurrentRestore()
    } else {
      activity.runOnUiThread { scheduleCurrentRestore() }
    }
  }

  fun remove(): Boolean {
    desired = false
    return onUiThreadConfirmed {
      generation += 1
      val current = button ?: return@onUiThreadConfirmed true
      val parent = current.parent
      if (parent != null && parent !is ViewGroup) return@onUiThreadConfirmed false
      (parent as? ViewGroup)?.removeView(current)
      button = null
      true
    }
  }

  private fun scheduleCurrentRestore() {
    val source = webView ?: return
    scheduleRestore(source, generation)
  }

  private fun scheduleRestore(source: WebView, expectedGeneration: Long) {
    source.post {
      if (
        desired &&
        generation == expectedGeneration &&
        webView === source &&
        source.isAttachedToWindow &&
        !activity.isDestroyed
      ) {
        installOnMainThread()
      }
    }
  }

  private fun installOnMainThread(): Boolean {
    val root = activity.findViewById<ViewGroup>(android.R.id.content) ?: return false
    val existing = button
    if (existing != null) {
      val parent = existing.parent
      if (parent !== root) {
        if (parent != null && parent !is ViewGroup) return false
        (parent as? ViewGroup)?.removeView(existing)
        root.addView(existing, controlLayoutParams())
      }
      existing.isEnabled = true
      existing.text = "Use Local Backend"
      existing.bringToFront()
      return true
    }
    val control = Button(activity).apply {
      text = "Use Local Backend"
      contentDescription = "Use Local Backend"
      isAllCaps = false
      setOnClickListener {
        if (requestLocalBackend()) {
          isEnabled = false
          text = "Switching…"
        }
      }
    }
    root.addView(
      control,
      controlLayoutParams(),
    )
    button = control
    control.bringToFront()
    return true
  }

  private fun controlLayoutParams(): FrameLayout.LayoutParams {
    val density = activity.resources.displayMetrics.density
    return FrameLayout.LayoutParams(
      FrameLayout.LayoutParams.WRAP_CONTENT,
      (48 * density).toInt(),
      Gravity.TOP or Gravity.END,
    ).apply {
      marginEnd = (12 * density).toInt()
      topMargin = (48 * density).toInt()
    }
  }

  private fun onUiThreadConfirmed(action: () -> Boolean): Boolean {
    if (Looper.myLooper() == Looper.getMainLooper()) return action()
    val latch = CountDownLatch(1)
    var result = false
    activity.runOnUiThread {
      result = action()
      latch.countDown()
    }
    return latch.await(2, TimeUnit.SECONDS) && result
  }
}
