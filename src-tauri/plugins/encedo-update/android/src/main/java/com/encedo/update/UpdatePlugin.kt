package com.encedo.update

import android.app.Activity
import android.content.Intent
import android.net.Uri
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.android.play.core.appupdate.AppUpdateManagerFactory
import com.google.android.play.core.install.model.AppUpdateType
import com.google.android.play.core.install.model.UpdateAvailability

/**
 * What Play knows about a newer release, and the flow that installs it.
 *
 * Play answers with a version code, a priority the publisher set on the release
 * (0-5) and how long the phone has been behind — but with no words. What was
 * wrong lives in the release notes, so the app's own screen says "a fix is
 * waiting, here is what for" and sends the person to Play to read it.
 *
 * None of this answers for a build that did not come from Play: a sideloaded
 * APK gets `installed_from_play = false` and the app falls back to opening the
 * store page.
 */
@TauriPlugin
class UpdatePlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val FLOW_REQUEST = 4711
        /** Play's own installers, plus the ones that keep Play's update path working. */
        private val PLAY_INSTALLERS = setOf("com.android.vending", "com.google.android.feedback")
    }

    private val manager by lazy { AppUpdateManagerFactory.create(activity) }

    /** The build this phone is running, as Play counts it. */
    private fun currentVersionCode(): Long {
        return try {
            val info = activity.packageManager.getPackageInfo(activity.packageName, 0)
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.P) {
                info.longVersionCode
            } else {
                @Suppress("DEPRECATION")
                info.versionCode.toLong()
            }
        } catch (e: Exception) {
            0L
        }
    }

    private fun installedFromPlay(): Boolean {
        val pm = activity.packageManager
        val installer = try {
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                pm.getInstallSourceInfo(activity.packageName).installingPackageName
            } else {
                @Suppress("DEPRECATION")
                pm.getInstallerPackageName(activity.packageName)
            }
        } catch (e: Exception) {
            null
        }
        return installer in PLAY_INSTALLERS
    }

    /** What Play has for this phone right now. */
    @Command
    fun check(invoke: Invoke) {
        val fromPlay = installedFromPlay()
        manager.appUpdateInfo
            .addOnSuccessListener { info ->
                val out = JSObject()
                out.put("available", info.updateAvailability() == UpdateAvailability.UPDATE_AVAILABLE)
                out.put("inProgress", info.updateAvailability() == UpdateAvailability.DEVELOPER_TRIGGERED_UPDATE_IN_PROGRESS)
                out.put("versionCode", info.availableVersionCode())
                // The number the publisher set on the release in the Play Console.
                out.put("priority", info.updatePriority())
                out.put("staleDays", info.clientVersionStalenessDays() ?: -1)
                out.put("immediateAllowed", info.isUpdateTypeAllowed(AppUpdateType.IMMEDIATE))
                out.put("installedFromPlay", fromPlay)
                out.put("currentVersionCode", currentVersionCode())
                invoke.resolve(out)
            }
            .addOnFailureListener { e ->
                // No Play services, no Play install, no network: not an error the
                // person can do anything about, so it is reported as a state.
                val out = JSObject()
                out.put("available", false)
                out.put("inProgress", false)
                out.put("versionCode", 0)
                out.put("priority", 0)
                out.put("staleDays", -1)
                out.put("immediateAllowed", false)
                out.put("installedFromPlay", fromPlay)
                out.put("currentVersionCode", currentVersionCode())
                out.put("error", e.message ?: e.javaClass.simpleName)
                invoke.resolve(out)
            }
    }

    /** Play's own full-screen update, the one a person cannot walk away from. */
    @Command
    fun start(invoke: Invoke) {
        manager.appUpdateInfo
            .addOnSuccessListener { info ->
                if (info.updateAvailability() != UpdateAvailability.UPDATE_AVAILABLE &&
                    info.updateAvailability() != UpdateAvailability.DEVELOPER_TRIGGERED_UPDATE_IN_PROGRESS
                ) {
                    invoke.reject("no_update: Play has nothing newer for this phone")
                    return@addOnSuccessListener
                }
                if (!info.isUpdateTypeAllowed(AppUpdateType.IMMEDIATE)) {
                    invoke.reject("not_allowed: Play will not run an immediate update here")
                    return@addOnSuccessListener
                }
                try {
                    @Suppress("DEPRECATION")
                    manager.startUpdateFlowForResult(info, AppUpdateType.IMMEDIATE, activity, FLOW_REQUEST)
                    invoke.resolve()
                } catch (e: Exception) {
                    invoke.reject("flow: ${e.message ?: e.javaClass.simpleName}")
                }
            }
            .addOnFailureListener { e -> invoke.reject("unavailable: ${e.message ?: e.javaClass.simpleName}") }
    }

    /** The store page, for a build Play cannot update in place. */
    @Command
    fun openStore(invoke: Invoke) {
        val id = activity.packageName
        for (uri in listOf("market://details?id=$id", "https://play.google.com/store/apps/details?id=$id")) {
            try {
                activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(uri)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                invoke.resolve()
                return
            } catch (e: Exception) {
                continue
            }
        }
        invoke.reject("no_store: nothing on this phone opens the store")
    }
}
