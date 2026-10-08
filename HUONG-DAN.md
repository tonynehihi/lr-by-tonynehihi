# Lightroom by Tony — Hướng dẫn cài đặt sạch & sử dụng

Bản build cá nhân của Tony, dựa trên LightCraft (MIT/Apache-2.0), đã thêm:
- Export **JPEG Ultra HDR** (có thanh HDR boost 0–4) để đăng Instagram lên HDR — **bản này viết lại gain map theo đúng chuẩn Lightroom** (3 kênh, full-res)
- **Bấm vào số** bên phải slider để gõ số trực tiếp (Enter = chốt, Esc = hủy)
- Giao diện **sáng/tối**: vào Settings (⌘,) → Interface → Theme
- Icon **LrT** phong cách Adobe, tên app "Lightroom by Tony"

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

## Gặp lỗi

Chụp màn hình lỗi gửi tao.
