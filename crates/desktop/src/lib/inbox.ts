import { fetch } from "@tauri-apps/plugin-http";
// fetch last inbox question id, store it if it isn't defined and then check if it
// it is different each time afterwards; send system notif if last inbox question id is different
