import "./services/loader.js";
import "./services/app.js";

import "./services/account_warnings.js";
import "./services/chats.js";
import "./services/comments.js";
import "./services/dialogs.js";
import "./services/items.js";
import "./services/mail.js";
import "./services/notifications.js";
import "./services/questions.js";
import "./services/reactions.js";
import "./services/reports.js";
import "./services/responses.js";
import "./services/search.js";
import "./services/tokens.js";
import "./services/warnings.js";

import "./services/classes/PartialComponent.js";
import "./services/carp.js";
import "./services/codemirror.js";

export default function init() {
    const app = ns("app");
    app.disconnect_observers();
    app.clean_date_codes();
    app.link_filter();

    app["hook.scroll"](document.body, document.documentElement);
    app["hook.character_counter.init"]();
    app["hook.long_text.init"]();
    app["hook.alt"]();
    app["hook.ips"]();
    app["hook.check_reactions"]();
    app["hook.tabs"]();
    app["hook.partial_embeds"]();
}
