// @ts-nocheck
(() => {
    const self = reg_ns("dialogs");

    class Dialog {
        #id;
        #element;

        constructor(id) {
            this.#id = id;
            this.#element = document.getElementById(id);
        }

        open() {
            this.#element.showModal();
        }

        close() {
            this.#element.close();

            // run events
            for (const event of self._event_store.dialogs[this.#id]) {
                event();
            }
        }
    }

    self.define("add", function ({ $ }, id) {
        // init dialogs
        if (!$.dialogs) {
            $.dialogs = {};
        }

        // init event store
        if (!$._event_store) {
            $._event_store = {};
        }

        if (!$._event_store.dialogs) {
            $._event_store.dialogs = {};
        }

        // add dialog
        $.dialogs[id] = new Dialog(id);
    });

    self.define("get", function ({ $ }, id) {
        return $.dialogs[id];
    });

    self.define("event:confirm", function ({ $ }, id, callback) {
        if (!$._event_store.dialogs[id]) {
            $._event_store.dialogs[id] = [];
        }

        $._event_store.dialogs[id].push(callback);
    });
})();
