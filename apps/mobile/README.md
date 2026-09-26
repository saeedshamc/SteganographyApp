# Open Stego Mobile

کلاینت Flutter روی `stego-ffi` → `stego-core` — همان فرمت رمزنگاری‌شدهٔ دسکتاپ/CLI.

## فقط اندروید (مسیر اصلی)

راهنمای کامل و دقیق:

**[docs/ANDROID.md](../../docs/ANDROID.md)**

خلاصهٔ سریع از ریشهٔ ریپو:

```powershell
# 1) NDK + cargo-ndk باید نصب باشند (جزئیات در ANDROID.md)
.\scripts\build-mobile-native.ps1 -Android

# 2) اجرا / APK
cd apps\mobile
flutter pub get
flutter devices
flutter run -d <android-device-id>

# یا:
flutter build apk --release
```

## لوگو / آیکن

منبع برند:

- `assets/open-stego-icon-1024.png`
- `assets/open-stego-logo.png`

کپی داخل اپ: `apps/mobile/assets/branding/`.  
بازسازی لانچر:

```powershell
cd apps\mobile
dart run flutter_launcher_icons
```

## ویندوز (اختیاری، برای دیباگ UI روی PC)

```powershell
.\scripts\build-mobile-native.ps1
cd apps\mobile
flutter run -d windows
```

## قابلیت‌ها

- **Hide** — cover + payload + پسورد + عمق LSB + plan
- **Extract** — بازیابی متن/فایل؛ Open/Run فقط با تأیید صریح
- **About** — وضعیت پل native + لوگو

نسخه: `pubspec.yaml` → `0.4.1+41` (هم‌تراز workspace `0.4.1`).
