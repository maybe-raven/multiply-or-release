echo "Building a MacOS universal app."
# set the name of the Mac App
set app_name "Multiply or Release"
# set the name of your rust crate
set rust_crate_name multiply_or_release
# create the folder structure
mkdir -p "$app_name.app/Contents/MacOS"
mkdir -p "$app_name.app/Contents/Resources"
# copy Info.plist
cp Info.plist "$app_name.app/Contents/Info.plist"
# copy the icon (assuming you already have it in Apple ICNS format)
cp AppIcon.icns "$app_name.app/Contents/Resources/AppIcon.icns"
# copy your Bevy game assets (not necessary)
# cp -a assets "$app_name.app/Contents/MacOS/"
# compile the executables for each architecture
cargo build --release --target x86_64-apple-darwin # build for Intel
cargo build --release --target aarch64-apple-darwin # build for Apple Silicon
# combine the executables into a single file and put it in the bundle
lipo "target/x86_64-apple-darwin/release/$rust_crate_name" \
    "target/aarch64-apple-darwin/release/$rust_crate_name" \
    -create -output "$app_name.app/Contents/MacOS/$app_name"
