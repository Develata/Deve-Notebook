package dev.deve.notebook.mobile

import android.os.Bundle
import android.os.Looper
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.FrameLayout
import androidx.activity.enableEdgeToEdge
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class MainActivity : TauriActivity() {
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
}
