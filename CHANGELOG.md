# Changelog — Lightroom by Tony

Mọi thay đổi đáng chú ý của bản fork cá nhân này. Cùng nội dung được đăng ở GitHub Releases và hiện trong app khi có bản mới.

## 1.1.6

- **Icon chuẩn macOS**: icon LrT được bo góc squircle và có đệm trong suốt đúng lưới icon macOS — hết cảnh vuông full viền trên Dock.
- **Đọc được file HEIC** (bản macOS): ảnh .heic từ iPhone import, xem và chỉnh sửa như ảnh thường, nhờ bộ giải mã HEIC có sẵn trong macOS. (Bản Windows chưa đọc được HEIC.)
- **Ảnh mất file gốc có pop-up**: mở một ảnh để chỉnh mà file gốc đã bị di chuyển hoặc xóa ở ngoài app, app hiện pop-up báo rõ, kèm nút:
  - **Locate File...** — chọn lại vị trí mới của file; app kiểm chứng đúng ảnh bằng nội dung file (file trùng tên nhưng khác ảnh sẽ bị từ chối, không gắn nhầm).
  - **Remove from Library** — gỡ hẳn ảnh khỏi thư viện.
  - **Not Now** — để sau.
- **Copy Settings hết nhầm lẫn**: chọn nhiều ảnh có settings khác nhau thì không copy được nữa (tránh không biết đang copy settings của ảnh nào). Chọn 1 ảnh, hoặc các ảnh đã chọn có settings giống nhau, thì copy bình thường.
- **Mở app là hỏi update**: có bản mới thì hiện pop-up kèm mục **What's new** (changelog này) để đọc trước khi quyết; không có bản mới thì không hiện gì cả. Bấm **Later** thì lần mở sau sẽ hỏi lại.
- Từ bản này, mọi bản mới đều có **changelog** ở GitHub Release và ngay trong app.

## 1.1.5

- **Tự kiểm tra & cài bản mới ngay trong app**: Help → Check for Updates (hoặc About → Updates). Có bản mới: Download Update → Quit & Install, app tự build và thay bản cũ trên macOS.

## 1.1.4

- Chọn nhiều ảnh trong filmstrip (vòng tròn tick, ⌘+click, Shift+click, nút All/None); Copy Settings / Paste Settings tách thành 2 nút riêng; Paste áp cho toàn bộ ảnh đã chọn; Export theo ảnh đã chọn.
