# VnKey

Bộ gõ tiếng Việt siêu nhẹ cho macOS, viết bằng Rust. Chỉ có **Telex** và **Unicode dựng sẵn**.

- Icon `V` / `E` trên menu bar, không hiện Dock
- Bật/tắt: nhấn rồi thả **Ctrl+Shift** (giống Unikey), hoặc bấm vào menu
- Gõ không gạch chân, dùng CGEventTap + Backspace (cùng cách với OpenKey, EVKey, GõKey)
- Tự trả lại nguyên văn khi gõ từ không phải tiếng Việt (`what`, `windows`...)
- Gõ lặp để huỷ: `ass` → `as`, `aaa` → `aa`, `ww` → `w`

## Cấu trúc

```
engine/   Bộ xử lý Telex thuần Rust, không phụ thuộc OS, có unit test
app/      App macOS: event tap (src/mac/tap.rs) + menu bar (src/mac/menu.rs)
bundle/   Info.plist và script đóng gói VnKey.app
```

## Build

Test engine (chạy được trên mọi OS):

```bash
cargo test -p vnkey-engine
```

Build app (trên macOS):

```bash
./bundle/build-app.sh    # -> target/VnKey.app
./bundle/build-dmg.sh    # -> target/VnKey-0.1.0.dmg (kéo vào Applications)
```

Lần đầu chạy script trên Mac: `chmod +x bundle/*.sh`.

Mở file `.dmg` trên máy khác: vì app chỉ ký ad-hoc (chưa notarize), macOS sẽ chặn ở lần mở đầu. Chuột phải vào VnKey → **Open**, hoặc chạy `xattr -dr com.apple.quarantine /Applications/VnKey.app`.

Lần chạy đầu, macOS sẽ hỏi quyền **Accessibility**. Vào System Settings → Privacy & Security → Accessibility, bật VnKey, rồi thoát và mở lại app.

> Mỗi lần build lại, chữ ký ad-hoc thay đổi nên macOS có thể đòi cấp quyền lại. Khi đó gỡ VnKey khỏi danh sách Accessibility rồi thêm lại.

## Tuỳ chỉnh

- Đặt dấu kiểu mới (`hoà`, `thuý`): đặt `engine.modern_tone = true` trong `app/src/mac/tap.rs`.

## Việc còn có thể làm

- Thanh địa chỉ Chrome/Safari có autocomplete đôi khi nuốt Backspace. OpenKey xử lý bằng cách gửi Shift+← để chọn chữ rồi gõ đè.
- Reset từ khi đổi ứng dụng (lắng nghe `NSWorkspaceDidActivateApplicationNotification`).
- Danh sách app loại trừ (Terminal, game...).
- Tự khởi động cùng máy (`SMAppService`).
