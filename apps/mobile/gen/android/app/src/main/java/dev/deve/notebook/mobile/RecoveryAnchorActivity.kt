package dev.deve.notebook.mobile

import android.app.Activity
import android.content.Intent
import android.os.Process

// Capability-free Android lifecycle anchor used only while RemoteBrowser is
// retired and the fresh bundled-local MainActivity is created.
class RecoveryAnchorActivity : TauriActivity() {
  fun scheduleBackendRecoveryColdStart(): Boolean = scheduleDeveColdStart()
}

internal fun Activity.scheduleDeveColdStart(): Boolean {
  val intent = Intent(this, BackendRecoveryRestartActivity::class.java).apply {
    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    putExtra(BackendRecoveryRestartActivity.EXTRA_PREVIOUS_PID, Process.myPid())
  }
  startActivity(intent)
  return true
}
