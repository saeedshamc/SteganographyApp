# شروع کار — بستهٔ اجرایی فاز ۰

با گفتن **«شروع کن»** همین سند پیاده می‌شود. جزئیات بلندمدت در `PHASE2_PLAN.md` است؛ اینجا فقط کارِ فوری و قابل‌اتمام است.

---

## هدف این اسپرینت

بتوانی یک **برنامه** را داخل عکس/PDF مخفی کنی، cover را مثل قبل باز کنی، بعد در Open Stego استخراج کنی و با تأیید واضح Save یا Run کنی — همه شفاف و آموزشی.

---

## محدوده (داخل / خارج)

| داخل | خارج از این اسپرینت |
|------|---------------------|
| `payload_kind`: Text / File / Executable | LSB تطبیقی، پروفایل Argon2، کلیدفایل |
| تشخیص اجرایی از پسوند (و انتخاب دستی در UI) | auto-run با دبل‌کلیک در Acrobat/Photos |
| GUI: بعد از extract → Save / Run با تأیید | Batch، Lab کامل، Flutter |
| CLI: `--run --i-understand` | CI کامل |
| ویزارد Demo ساده (چند مرحلهٔ متنی در GUI) | format-aware PDF پیشرفته |
| فیکسچر دمو بی‌خطر + تست | |
| به‌روز `LEARNING.md` + اشاره در README | |

---

## مراحل کار (به ترتیب)

### ۱) هسته — `stego-core`

- فیلد `kind` به `PayloadMeta` (سازگار با فایل‌های قدیمی: پیش‌فرض `File` / `Text`)
- `for_executable(...)` + تشخیص از پسوند (`.exe`, `.msi`, `.bat`, `.cmd`, `.sh`, `.AppImage`, …)
- به‌روز `FORMAT.md` در صورت نیاز به بایت جدید؛ اگر بدون شکستن v1 بشود، همان نسخه بماند
- تست round-trip برای kind=Executable

**Commit:** `add payload kind for executable demo payloads`

### ۲) CLI — `stego-cli`

- Hide: اگر فایل اجرایی است، kind را Executable بگذارد
- Extract: چاپ kind؛ فلگ‌های `--run` و `--i-understand`
- بدون هر دو فلگ → فقط ذخیره؛ با هر دو → بعد از extract اجرا (مسیر خروجی)

**Commit:** `allow optional run after extracting executable via CLI`

### ۳) GUI — Tauri desktop

- Hide: انتخاب نوع payload (متن / فایل / برنامه) یا auto از پسوند
- Extract: اگر Executable → دیالوگ: نام، اندازه، hash کوتاه + **Save** / **Run**
- Run فقط بعد از تأیید دوم با متن شفاف آموزشی
- تب یا پنل کوتاه **Demo**: ۳–۴ قدم توضیح مسیر (بدون پیچیدگی زیاد)

**Commit:** `add transparent extract-and-run flow in desktop UI`

### ۴) فیکسچر + اسناد

- یک payload دمو بی‌خطر (مثلاً اسکریپت/`echo` یا باینری تست کوچک در `test-fixtures/demo/`)
- بخش کوتاه در `LEARNING.md`: مسیر Demo + «چرا بینندهٔ سیستم payload را اجرا نمی‌کند»
- یک پاراگراف در README که به Demo اشاره کند

**Commit:** `document executable demo flow for learners`

---

## معیار «تموم شد»

- [ ] hide یک فایل با پسوند اجرایی داخل PNG یا PDF
- [ ] extract با GUI و CLI درست برمی‌گردد و `kind` شناخته می‌شود
- [ ] Run فقط با تأیید / فلگ‌های صریح
- [ ] دبل‌کلیک روی cover در OS برنامه را اجرا نمی‌کند (رفتار درست)
- [ ] تست‌های هسته سبز
- [ ] چهار commit جدا با پیام انسانی (طبق عادت پروژه)

---

## بعد از این اسپرینت (فاز A — جداگانه)

پروفایل Fast/Balanced/Paranoid، LSB تطبیقی، کلیدفایل — فقط وقتی فاز ۰ تمام و تأیید شد.

---

## دستور شروع

کافی است بنویسی:

**شروع کن**

یا دقیق‌تر: **فاز ۰ را پیاده کن**
