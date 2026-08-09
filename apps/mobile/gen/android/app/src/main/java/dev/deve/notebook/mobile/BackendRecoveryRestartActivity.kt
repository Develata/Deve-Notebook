package dev.deve.notebook.mobile

import android.app.Activity
import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.Process
import android.os.SystemClock

// Runs in a dedicated process so the launcher task can be created only after
// the prior Tauri process has been proven absent.
class BackendRecoveryRestartActivity : Activity() {
  internal companion object {
    const val EXTRA_PREVIOUS_PID = "dev.deve.notebook.mobile.PREVIOUS_PID"
    const val PREVIOUS_PROCESS_RETIRE_TIMEOUT_MS = 10_000L
    const val RETIRE_POLL_MS = 100L
    const val REQUIRED_ABSENT_SAMPLES = 2
  }

  private val handler = Handler(Looper.getMainLooper())
  private var previousPid = 0
  private var deadlineMs = 0L
  private var consecutiveAbsentSamples = 0

  private val retirementProbe = object : Runnable {
    override fun run() {
      if (previousProcessIsAlive()) {
        consecutiveAbsentSamples = 0
      } else {
        consecutiveAbsentSamples += 1
      }
      if (consecutiveAbsentSamples >= REQUIRED_ABSENT_SAMPLES) {
        launchColdStart()
        return
      }
      if (SystemClock.elapsedRealtime() >= deadlineMs) {
        finishRecoveryProcess()
        return
      }
      handler.postDelayed(this, RETIRE_POLL_MS)
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    previousPid = intent.getIntExtra(EXTRA_PREVIOUS_PID, 0)
    if (previousPid <= 0 || previousPid == Process.myPid()) {
      finishRecoveryProcess()
      return
    }
    deadlineMs = SystemClock.elapsedRealtime() + PREVIOUS_PROCESS_RETIRE_TIMEOUT_MS
    handler.post(retirementProbe)
  }

  override fun onDestroy() {
    handler.removeCallbacks(retirementProbe)
    super.onDestroy()
  }

  private fun previousProcessIsAlive(): Boolean {
    val activityManager = getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
    val processes = activityManager.runningAppProcesses ?: return true
    return processes.any { process -> process.pid == previousPid }
  }

  private fun launchColdStart() {
    val component = packageManager.getLaunchIntentForPackage(packageName)?.component
      ?: return finishRecoveryProcess()
    startActivity(Intent.makeRestartActivityTask(component))
    finishRecoveryProcess()
  }

  private fun finishRecoveryProcess() {
    finishAndRemoveTask()
    Process.killProcess(Process.myPid())
  }
}
