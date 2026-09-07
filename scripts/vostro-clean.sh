#!/usr/bin/env bash
# Stop everything a build session leaves behind on vostro: the emulator (which
# burns CPU while idle, and far more with the camera open), adb, the Gradle
# daemon and the Kotlin compile daemons. Run at the end of every session:
#   ssh vostro 'bash ~/develop/encedo-authenticator/scripts/vostro-clean.sh'
# Leaves the Windows VM (qemu-system-x86_64, started at boot) and desktop apps alone.
set -u
source ~/.android-env.sh 2>/dev/null || true
adb emu kill >/dev/null 2>&1 && sleep 3
adb kill-server >/dev/null 2>&1
for d in ~/develop/encedo-authenticator/src-tauri/gen/android ~/develop/encedo-authenticator-dev/src-tauri/gen/android; do
  [ -x "$d/gradlew" ] && (cd "$d" && ./gradlew --stop -q >/dev/null 2>&1)
done
# Whatever survived the polite requests. Patterns avoid matching this shell's own command line.
pkill -f 'qemu-system-x86_64-headles[s]' 2>/dev/null
pkill -f 'emulator/netsim[d]|emulator/crashpad_handle[r]|KotlinCompileDaemo[n]|GradleDaemo[n]|GradleWorkerMai[n]' 2>/dev/null
sleep 1
echo "left over (should be empty):"
ps -eo pid,etimes,rss,args --sort=-rss | grep -iE 'qemu-system-x86_64-headles[s]|emulator/|GradleDaemo[n]|KotlinCompileDaemo[n]|adb -L' | cut -c1-120
free -g | sed -n 2p
