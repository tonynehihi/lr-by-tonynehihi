# Lightroom by Tony — Hướng dẫn cài đặt sạch & sử dụng

Bản build cá nhân của Tony, dựa trên LightCraft (MIT/Apache-2.0), đã thêm:
- Export **JPEG Ultra HDR** (có thanh HDR boost 0–4) để đăng Instagram lên HDR — **bản này viết lại gain map theo đúng chuẩn Lightroom** (3 kênh, full-res)
- **Bấm vào số** bên phải slider để gõ số trực tiếp (Enter = chốt, Esc = hủy)
- Giao diện **sáng/tối**: vào Settings (⌘,) → Interface → Theme
- Icon **LrT** phong cách Adobe, tên app "Lightroom by Tony"

## Mới ở bản 1.1.7 — dọn menu Help, bỏ hết link về LightCraft gốc

- Menu **Help** giờ chỉ còn 4 mục: **About Lightroom by Tony**, **Check for Updates**, **System Info**, **Keyboard Shortcuts**.
- Đã xóa toàn bộ mục dẫn về dự án gốc: mục Help (tài liệu của bản gốc), Join the ArtCraft Discord, Send Feedback, Lightroom by Tony Website, Lightroom by Tony on GitHub (mục này trỏ về repo gốc storytold/lightcraft chứ không phải repo của Tony), ArtCraft Website, và **What's New** (nội dung là changelog của bản gốc). Changelog của bản này vẫn nằm ở pop-up update và trang GitHub Releases của Tony như cũ.

## Mới ở bản 1.1.6 — HEIC, pop-up file mất, hỏi update khi mở app

- **Icon chuẩn macOS**: icon LrT bo góc squircle + đệm trong suốt đúng lưới icon macOS, hết vuông full viền trên Dock.
- **Đọc được file HEIC** (bản macOS): ảnh .heic từ iPhone import, xem và chỉnh như ảnh thường (app mượn bộ giải mã HEIC có sẵn trong macOS; bản Windows chưa đọc được HEIC).
- **Ảnh mất file gốc**: mở ảnh để chỉnh mà file gốc đã bị di chuyển/xóa ở ngoài app → hiện pop-up: **Locate File...** (chọn lại vị trí mới — app kiểm chứng đúng ảnh bằng nội dung, file trùng tên nhưng khác ảnh bị từ chối), **Remove from Library** (gỡ hẳn khỏi thư viện), **Not Now** (để sau).
- **Copy Settings**: chọn nhiều ảnh có settings khác nhau thì không copy được nữa, tránh nhầm lẫn đang copy settings của ảnh nào. Chọn 1 ảnh, hoặc các ảnh settings giống nhau, copy bình thường.
- **Mở app hỏi update**: có bản mới mới hiện pop-up, kèm **What's new** (changelog) để đọc trước; không có bản mới thì không hiện gì. Bấm **Later** thì lần mở sau hỏi lại. Cài bản này bằng chính nút update trong app 1.1.5: **Help → Check for Updates**.

## Mới ở bản 1.1.5 — tự kiểm tra & cài bản mới (Check for Updates)

- Vào **Help → Check for Updates** (hoặc mở **About Lightroom by Tony**, mục **Updates** ở dưới cùng): app tự hỏi GitHub xem có bản mới hơn không.
- Có bản mới: bấm **Download Update** để tải về, rồi **Quit & Install** — app tự thoát, build bản mới (vài phút, chạy ngầm), thay app trong Applications rồi tự mở lại. **Đây là lần cuối phải cài tay**: từ bản sau (1.1.6...) cứ bấm nút trong app là xong.
- Nếu cài đặt dở dang có trục trặc: xem log ở `~/Library/Logs/LightroomByTony-update.log`. Build lỗi thì app cũ vẫn nguyên vẹn và tự mở lại, không mất gì.
- Bản Windows: kiểm tra bản mới trong About được, nhưng chưa tự cài — tải bản mới ở trang release (nút **Open Release Page** trong app).

## Mới ở bản 1.1.4 — chọn nhiều ảnh + Copy/Paste tách nút

- Thanh dưới có **2 nút riêng**: **Copy Settings** và **Paste Settings** (trước đây gộp 1 nút, copy xong là mất nút copy). Copy lúc nào cũng được, không cần paste trước.
- **Chọn nhiều ảnh trong filmstrip**: bấm **vòng tròn tick** ở góc trên-phải mỗi ảnh thu nhỏ để chọn/bỏ chọn (ảnh đã chọn có dấu ✓ xanh). Hoặc: **⌘+click** = chọn thêm/bỏ chọn, **Shift+click** = chọn cả loạt từ ảnh đang chọn tới ảnh vừa bấm.
- Nút **All / None** ở thanh dưới: chọn tất cả ảnh đang hiển thị / bỏ chọn hết.
- **Paste Settings** sẽ dán vào **toàn bộ ảnh đã chọn** (không chỉ ảnh đang mở). Export (File → Export) cũng xuất đúng những ảnh đã chọn.

## Gỡ bản cũ / cài lại sạch

**Nếu mày đã build bản trước:**
```bash
# Xóa bản build cũ
rm -rf target/
# Xóa app đã cài (nếu đã kéo vào Applications)
rm -rf "/Applications/Lightroom by Tony.app"
rm -rf "/Applications/LightCraft.app"
```

**Xóa sạch thư viện/cache (nếu muốn làm lại từ đầu):**
```bash
# Thư viện ảnh (CẨN THẬN: xóa là mất catalog, sao lưu trước!)
rm -rf ~/Pictures/LightCraft\ Library
# Cache & settings
rm -rf ~/Library/Application\ Support/lightcraft
rm -rf ~/.config/lightcraft
```

## Cài đặt (làm 1 lần)

**Bước 1** — Cài Xcode Command Line Tools (nếu chưa có):
```
xcode-select --install
```

**Bước 2** — Cài Rust (nếu chưa có):
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Xong thì **đóng Terminal, mở lại**.

## Build & chạy

**Bước 3** — Giải nén `lightroom-by-tony.zip`.

**Bước 4** — Trong Terminal:
```bash
cd <kéo folder lightroom-by-tony vào Terminal rồi thả>
cargo build --release -p lightcraft
```
(Lần đầu 10–20 phút.)

**Bước 5** — Chạy thử:
```bash
./target/release/lightcraft
```

**Bước 6** — Đóng gói thành app (không cần Terminal nữa):
```bash
packaging/macos/package.sh --arch aarch64
```
(Máy Intel cũ: `--arch x86_64`.)
Xong mở file `.dmg` trong `dist/release/`, kéo **Lightroom by Tony.app** vào Applications.

## Dùng HDR cho Instagram

1. Chỉnh ảnh như bình thường.
2. File → Export → Format: **JPEG Ultra HDR**.
3. Kéo **HDR boost** (thử 1.5 trước).
4. Export → đăng Instagram.

## Zoom ảnh bằng trackpad

Dùng **2 ngón tay pinch** trên trackpad để zoom to/nhỏ ảnh (như Photos).
Kéo 1 ngón để di chuyển ảnh khi đã zoom.

## Đổi giao diện sáng/tối

Nhấn `⌘,` (Settings) → tab **Interface** → mục **Theme** → chọn Dark hoặc Light.

## App không mở được (macOS chặn app tự đóng gói)

App tự đóng gói không có chữ ký Apple nên macOS hay chặn — không phải app hỏng.

**Trường hợp 1** — macOS báo app "from an unidentified developer":
Chuột phải vào app → chọn **Open** → bấm **Open** trong hộp thoại hiện ra.

**Trường hợp 2** — macOS báo app "is damaged" hoặc "can't be opened":
1. Mở Terminal.
2. Gõ đúng dòng này rồi Enter:
```bash
xattr -cr "/Applications/Lightroom by Tony.app"
```
3. Mở app lại bình thường.

**Trường hợp 3** — app crash / tắt ngay khi mở:
Kiểm tra lại **Bước 6** đã đúng chip máy chưa — máy Apple Silicon (M1/M2/M3/M4) phải dùng `--arch aarch64`, máy Intel cũ dùng `--arch x86_64`. Đóng gói sai chip là crash luôn, phải đóng gói lại.

## Build báo lỗi

- Thiếu `xcode-select`: chạy `xcode-select --install` rồi build lại.
- Terminal không nhận lệnh `cargo`: đóng Terminal, mở lại (Rust mới thêm vào PATH sau khi cài).
- Lỗi khác: chụp màn hình lỗi gửi tao.
