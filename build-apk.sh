#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "Computing build version..."
echo ""
PRODUCT_VERSION=$(cargo run -q --bin mullvad-version versionName)
echo "Building Mullvad VPN $PRODUCT_VERSION for Android"
echo ""

BUILD_TYPE="release"
GRADLE_BUILD_FLAVORS="full"
GRADLE_TASKS="assembleFullRelease"
BUNDLE_TASK="bundlePlayRelease"
CARGO_ARGS="--release"
EXTRA_WGGO_ARGS=""
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"target"}
SKIP_STRIPPING=${SKIP_STRIPPING:-"no"}

while [ ! -z "${1:-""}" ]; do
    if [[ "${1:-""}" == "--dev-build" ]]; then
        BUILD_TYPE="debug"
        GRADLE_TASKS="assembleFullDebug"
        FILE_SUFFIX="-debug"
        CARGO_ARGS="--features api-override"
    elif [[ "${1:-""}" == "--fdroid" ]]; then
        GRADLE_BUILD_FLAVORS="fdroid"
        GRADLE_TASKS="assembleFdroidRelease"
        EXTRA_WGGO_ARGS="--no-docker"
    elif [[ "${1:-""}" == "--app-bundle" ]]; then
        GRADLE_BUILD_FLAVORS="full play"
        GRADLE_TASKS="assembleFullRelease assemblePlayRelease"
    elif [[ "${1:-""}" == "--no-docker" ]]; then
        EXTRA_WGGO_ARGS="--no-docker"
    elif [[ "${1:-""}" == "--skip-stripping" ]]; then
        SKIP_STRIPPING="yes"
    fi

    shift 1
done

if [[ "$BUILD_TYPE" == "release" && "$GRADLE_BUILD_FLAVORS" != "fdroid" ]]; then
    if [ ! -f "$SCRIPT_DIR/android/credentials/keystore.properties" ]; then
        echo "ERROR: No keystore.properties file found" >&2
        echo "       Please configure the signing keys as described in the README" >&2
        exit 1
    fi
fi

if [[ "$BUILD_TYPE" == "release" && "$PRODUCT_VERSION" != *"-dev-"* ]]; then
    echo "Removing old Rust build artifacts"
    cargo clean
    CARGO_ARGS+=" --locked"
fi

pushd "$SCRIPT_DIR/android"

# Fallback to the system-wide gradle command if the gradlew script is removed.
# It is removed by the F-Droid build process before the build starts.
if [ -f "gradlew" ]; then
    GRADLE_CMD="./gradlew"
elif which gradle > /dev/null; then
    GRADLE_CMD="gradle"
else
    echo "ERROR: No gradle command found" >&2
    echo "       Please either install gradle or restore the gradlew file" >&2
    exit 2
fi

$GRADLE_CMD --console plain clean
mkdir -p "app/build/extraJni"
popd

./wireguard/build-wireguard-go.sh --android $EXTRA_WGGO_ARGS

for ARCHITECTURE in ${ARCHITECTURES:-aarch64 armv7 x86_64 i686}; do
    case "$ARCHITECTURE" in
        "x86_64")
            TARGET="x86_64-linux-android"
            ABI="x86_64"
            ;;
        "i686")
            TARGET="i686-linux-android"
            ABI="x86"
            ;;
        "aarch64")
            TARGET="aarch64-linux-android"
            ABI="arm64-v8a"
            ;;
        "armv7")
            TARGET="armv7-linux-androideabi"
            ABI="armeabi-v7a"
            ;;
    esac

    echo "Building mullvad-daemon for $TARGET"
    cargo build $CARGO_ARGS --target "$TARGET" --package mullvad-jni

    STRIP_TOOL="${NDK_TOOLCHAIN_DIR}/llvm-strip"
    TARGET_LIB_PATH="$SCRIPT_DIR/android/app/build/extraJni/$ABI/libmullvad_jni.so"
    UNSTRIPPED_LIB_PATH="$CARGO_TARGET_DIR/$TARGET/$BUILD_TYPE/libmullvad_jni.so"

    if [[ "$SKIP_STRIPPING" == "yes" ]]; then
        cp "$UNSTRIPPED_LIB_PATH" "$TARGET_LIB_PATH"
    else
        $STRIP_TOOL --strip-debug --strip-unneeded -o "$TARGET_LIB_PATH" "$UNSTRIPPED_LIB_PATH"
    fi
done

echo "Updating relays.json..."
cargo run --bin relay_list $CARGO_ARGS > build/relays.json

cd "$SCRIPT_DIR/android"


for TASK in $GRADLE_TASKS; do
    $GRADLE_CMD --console plain "$TASK"
done

mkdir -p "$SCRIPT_DIR/dist"

for FLAVOR in $GRADLE_BUILD_FLAVORS; do

    if [[ "$BUILD_TYPE" == "release" && "$FLAVOR" == "fdroid" ]]; then
        SOURCE_FILE_NAME="app-${FLAVOR}-${BUILD_TYPE}-unsigned"
    else
        SOURCE_FILE_NAME="app-${FLAVOR}-${BUILD_TYPE}"
    fi

    if [[ "$BUILD_TYPE" == "release" ]]; then
        FILE_SUFFIX=""
    else
        FILE_SUFFIX="-debug"
    fi

    # Skip flavor in name for the full archive since that is that is deemed the main/default release
    # artifact without any marketplace adaptions etc.
    if [[ "$FLAVOR" == "full" ]]; then
        TARGET_FILE_NAME="MullvadVPN-${PRODUCT_VERSION}${FILE_SUFFIX}"
    else
        TARGET_FILE_NAME="MullvadVPN-${PRODUCT_VERSION}${FILE_SUFFIX}.${FLAVOR}"
    fi

    if [[ "$FLAVOR" != "play" ]]; then
        cp  "$SCRIPT_DIR/android/app/build/outputs/apk/$FLAVOR/$BUILD_TYPE/$SOURCE_FILE_NAME.apk" \
            "$SCRIPT_DIR/dist/$TARGET_FILE_NAME.apk"
    else
        $GRADLE_CMD --console plain "$BUNDLE_TASK"

        cp  "$SCRIPT_DIR/android/app/build/outputs/bundle/$FLAVOR${BUILD_TYPE^}/$SOURCE_FILE_NAME.aab" \
            "$SCRIPT_DIR/dist/$TARGET_FILE_NAME.aab"
    fi

done

echo "**********************************"
echo ""
echo " The build finished successfully! "
echo " You have built:"
echo ""
echo " $PRODUCT_VERSION"
echo ""
echo "**********************************"
