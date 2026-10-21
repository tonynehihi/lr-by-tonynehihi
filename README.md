<p align="center">
  <img alt="Lightroom by Tony" src="assets/app-icon/lrt-icon-1024.png" width="180">
</p>

<h1 align="center">Lightroom by Tony</h1>

<p align="center">
  <b>Bản fork cá nhân của <a href="https://github.com/storytold/lightcraft">LightCraft</a> — chỉnh lại để Tony dùng cho nhu cầu riêng.</b><br>
  Personal fork of LightCraft by storytold, rebranded and customized for personal use only.
</p>

## Nguồn gốc

Toàn bộ mã nguồn gốc của dự án này là [LightCraft](https://github.com/storytold/lightcraft) của **storytold** — một ứng dụng thư viện ảnh và phát triển ảnh RAW viết bằng Rust, phát hành theo giấy phép **MIT / Apache-2.0** (xem `LICENSE-MIT` và `LICENSE-APACHE` trong repo, giữ nguyên theo bản gốc).

Repo này **không phải** LightCraft chính thức và cũng không phải một sản phẩm phát hành ra công chúng. Đây chỉ là bản sao Tony lấy về, đổi tên thành **Lightroom by Tony** và chỉnh thêm vài tính năng theo thói quen sử dụng cá nhân. Mọi công lao của phần mềm gốc thuộc về storytold và các cộng tác viên của LightCraft.

> "Lightroom" là thương hiệu của Adobe. Dự án cá nhân này không có bất kỳ liên quan hay xác nhận nào từ Adobe; tên gọi chỉ để dùng trong phạm vi cá nhân của Tony.

## Tony đã chỉnh những gì

- **Đổi tên & nhận diện**: tên app "Lightroom by Tony", icon **LrT**, menu About và bundle macOS riêng.
- **Export JPEG Ultra HDR** (gain map đúng chuẩn, có thanh HDR boost 0–4) để đăng Instagram nhận HDR thật.
- **Pinch-to-zoom** bằng trackpad hai ngón trong màn hình xem ảnh.
- **Giao diện sáng/tối** theo ý thích (Settings → Interface → Theme).
- **Chọn nhiều ảnh trong filmstrip** (vòng tròn tick, ⌘+click, Shift+click, nút All/None) và **Copy Settings / Paste Settings tách thành 2 nút riêng**; Paste áp cho toàn bộ ảnh đã chọn.
- **Tự cập nhật trong app** (từ bản 1.1.5): *Help → Check for Updates* — app kiểm tra bản mới nhất ngay tại mục Releases của repo này, tự tải về, build và cài đặt trên macOS.

## Cài đặt trên Mac

Bản Mac là bản source, tự build bằng Cargo trên máy của Tony. Tóm tắt:

```bash
cargo build --release -p lightcraft
packaging/macos/package.sh --arch aarch64
```

File DMG nằm trong `dist/release/`. Hướng dẫn đầy đủ (cài sạch, gỡ bản cũ, xử lý Gatekeeper) nằm ở `packaging/macos/HUONG-DAN.md`.

Từ bản **1.1.5** trở đi không cần làm lại các bước này mỗi lần có bản mới: cứ mở app và bấm **Check for Updates**.

## Giấy phép

Giữ nguyên giấy phép của dự án gốc: **MIT OR Apache-2.0**. Các chỉnh sửa của Tony trong repo này cũng được để dưới cùng điều kiện đó, kèm ghi nhận nguồn gốc như trên.
