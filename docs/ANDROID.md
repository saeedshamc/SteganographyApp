# Building Open Stego for Android (Flutter only)

این راهنما فقط مسیر **Flutter → Android** است (APK / App Bundle). دسکتاپ Tauri جدا است؛ همین فرمت روی دیسک از طریق `stego-ffi` → `stego-core` استفاده می‌شود.

## نتیجهٔ مورد انتظار

| خروجی | دستور |
|--------|--------|
| نصب روی گوشی/امولاتور | `flutter run` |
| APK دیباگ | `flutter build apk --debug` |
| APK ریلیز | `flutter build apk --release` |
| Play Store (AAB) | `flutter build appbundle --release` |

لوگو/آیکن لانچر از `assets/open-stego-icon-1024.png` ساخته می‌شود (نام نمایشی اپ: **Open Stego**).

---

## پیش‌نیازها (یک‌بار)

### 1) Flutter + Android toolchain

```powershell
flutter doctor -v
```

باید این‌ها سبز باشند:

- Flutter SDK
- **Android toolchain** (SDK + `platform-tools` + licenses)
- دستگاه واقعی یا امولاتور (`flutter devices`)

اگر لایسنس‌ها ناقص‌اند:

```powershell
flutter doctor --android-licenses
```

Android Studio توصیه می‌شود (SDK Manager از همان‌جا).

### 2) NDK (برای کتابخانهٔ native `stego-ffi`)

بدون NDK، Flutter اپ را می‌سازد ولی **Hide/Extract** بدون `.so` کار نمی‌کند.

از SDK Manager یا:

```powershell
$sdk = "$env:LOCALAPPDATA\Android\sdk"
& "$sdk\cmdline-tools\latest\bin\sdkmanager.bat" "ndk;27.0.12077973"
```

سپس:

```powershell
$env:ANDROID_NDK_HOME = "$env:LOCALAPPDATA\Android\sdk\ndk\27.0.12077973"
# یا آخرین پوشه زیر ndk\
```

(نسخهٔ NDK را با آنچه SDK Manager نصب کرده هم‌خوان کنید.)

### 3) Rust targets + cargo-ndk

```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-ndk
cargo ndk --version
```

---

## بیلد مرحله‌به‌مرحله (از ریشهٔ ریپو)

### گام A — native library برای Android

```powershell
cd D:\Saeed\GitHub\SteganographyApp
.\scripts\build-mobile-native.ps1 -Android
```

این کار:

1. `cargo build -p stego-ffi --release` (ویندوز host، برای خودتان)
2. با `cargo-ndk` خروجی می‌ریزد زیر:

```text
apps/mobile/android/app/src/main/jniLibs/arm64-v8a/libstego_ffi.so
apps/mobile/android/app/src/main/jniLibs/armeabi-v7a/libstego_ffi.so
```

این `.so`ها را commit نکنید (در `.gitignore` هستند)؛ روی هر ماشین قبل از `flutter build` دوباره بسازید.

### گام B — وابستگی‌های Flutter

```powershell
cd apps\mobile
flutter pub get
```

### گام C — اجرا روی دستگاه اندروید

گوشی را با USB Debugging وصل کنید، یا امولاتور را روشن کنید:

```powershell
flutter devices
flutter run -d <deviceId>
```

مثال با دستگاه واقعی که `flutter doctor` دیده:

```powershell
flutter run -d RZCTB1BTSJW
```

### گام D — فقط APK / AAB

```powershell
cd apps\mobile

# APK (نصب دستی)
flutter build apk --release

# مسیر معمول:
# build\app\outputs\flutter-apk\app-release.apk

# برای Google Play:
flutter build appbundle --release
# build\app\outputs\bundle\release\app-release.aab
```

قبل از هر release، حتماً دوباره `.\scripts\build-mobile-native.ps1 -Android` را زده باشید تا `.so` داخل APK باشد.

---

## لوگو و آیکن لانچر

| فایل منبع | نقش |
|-----------|------|
| `assets/open-stego-icon-1024.png` | آیکن مربع شفاف برای pipeline |
| `assets/open-stego-logo.png` | لوگوی برند (درباره / UI) |
| `apps/mobile/assets/branding/app_icon.png` | کپی آیکن برای Flutter |
| `apps/mobile/assets/branding/logo.png` | کپی لوگو داخل اپ |

تولید دوبارهٔ mipmap / adaptive icon / iOS:

```powershell
cd apps\mobile
# اگر آیکن ریشه عوض شد:
Copy-Item ..\..\assets\open-stego-icon-1024.png assets\branding\app_icon.png -Force
Copy-Item ..\..\assets\open-stego-logo.png assets\branding\logo.png -Force
flutter pub get
dart run flutter_launcher_icons
```

Adaptive icon اندروید: پس‌زمینه `#0B4F56` (teal برند) + همان فرام‌گراوند keyhole.

نام زیر لانچر: **Open Stego** (`AndroidManifest.xml` → `android:label`).

---

## بررسی اینکه native لود شده

در تب **About** باید چیزی شبیه این ببینید:

```text
native 0.4.x ← libstego_ffi.so
```

اگر نوشت `native lib not loaded` یعنی `.so` داخل `jniLibs` نیست یا ABI دستگاه جور نیست (مثلاً فقط `x86_64` امولاتور بدون بیلد آن ABI).

برای امولاتور x86_64 در صورت نیاز:

```powershell
rustup target add x86_64-linux-android
cargo ndk -t x86_64 -o apps\mobile\android\app\src\main\jniLibs build -p stego-ffi --release
```

---

## امضای ریلیز (خلاصه)

برای فروشگاه، keystore خودتان را بسازید و در `android/app/build.gradle.kts` / `key.properties` تنظیم کنید. فعلاً پیکربندی پیش‌فرض با **debug signing** است تا `flutter run --release` روی دستگاه شخصی کار کند — برای انتشار عمومی حتماً signing را عوض کنید.

---

## عیب‌یابی سریع

| مشکل | کار |
|------|-----|
| `flutter doctor` قرمز روی Android | SDK/licenses/Android Studio را کامل کنید |
| `cargo ndk` پیدا نمی‌شود | `cargo install cargo-ndk` |
| `linker` / NDK errors | `ANDROID_NDK_HOME` را به پوشهٔ نسخهٔ نصب‌شده ست کنید |
| Hide کار نمی‌کند / About بدون native | `-Android` را دوباره بزنید؛ APK را از نو بسازید |
| آیکن قدیمی روی گوشی | `dart run flutter_launcher_icons` سپس uninstall/reinstall اپ |
| فقط می‌خواهم UI بدون native ببینم | `flutter run` ممکن است بالا بیاید؛ Hide/Extract تا داشتن `.so` خطا می‌دهد |

---

## چک‌لیست قبل از دادن APK به کسی

1. `cargo test -p stego-core --lib`
2. `.\scripts\build-mobile-native.ps1 -Android`
3. `cd apps\mobile && flutter analyze && flutter test`
4. `flutter build apk --release`
5. روی گوشی: Hide → Extract یک PNG کوچک با پسورد تست
6. تأیید کنید نام و آیکن لانچر **Open Stego** + لوگوی keyhole است

نسخهٔ اپ از `pubspec.yaml` می‌آید (`0.4.1+41` = نام `0.4.1`، کد بیلد `41`) و باید با `workspace.package.version` در ریشه هم‌راستا بماند.
