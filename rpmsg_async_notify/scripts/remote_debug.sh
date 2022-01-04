VSCODE_WS="$1"
SSH_REMOTE="$2"
GDBPORT="$3"

APP="signal_hook"
TARGET_ARCH="aarch64-unknown-linux-gnu"
BUILD_BIN_FILE="${VSCODE_WS}/target/${TARGET_ARCH}/debug/examples/${APP}"
TARGET_USER="root"
TARGET_BIN_FILE="/home/root/${APP}"
TARGET_CWD="/home/root"

sshpass -p root ssh "${TARGET_USER}@${SSH_REMOTE}" "killall gdbserver ${APP}"

if ! rsync -avz "${BUILD_BIN_FILE}" "${TARGET_USER}@${SSH_REMOTE}:${TARGET_BIN_FILE}"; then
    # If rsync doesn't work, it may not be available on target. Fallback to trying SSH copy.
    if ! sshpass -p root scp "${BUILD_BIN_FILE}" "${TARGET_USER}@${SSH_REMOTE}:${TARGET_BIN_FILE}"; then
        exit 2
    fi
fi

sshpass -p root ssh -f "${TARGET_USER}@${SSH_REMOTE}" "sh -c 'cd ${TARGET_CWD}; nohup gdbserver *:${GDBPORT} ${TARGET_BIN_FILE} > /dev/null 2>&1 &'"