(() => {
    const self = reg_ns("reactions", ["app"]);

    self.define("create", function (_, id, type) {
        fetch(`/api/v1/reactions/${id}`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                type,
            }),
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app::toast", [
                    res.success ? "success" : "error",
                    res.message || "Reaction added!",
                ]);
            });
    });

    self.define("delete", function (_, id) {
        fetch(`/api/v1/reactions/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app::toast", [
                    res.success ? "success" : "error",
                    res.message || "Reaction removed!",
                ]);
            });
    });

    self.define("has-reacted", function (_, id) {
        return new Promise((resolve, _) => {
            fetch(`/api/v1/reactions/${id}`, {
                method: "GET",
            })
                .then((res) => res.json())
                .then((res) => {
                    return resolve(res.success);
                });
        });
    });

    self.define("toggle", async function ({ $, app }, id, type, target) {
        await app.debounce("reactions::toggle");
        const remove = (await $["has-reacted"](id)) === true;

        if (remove) {
            if (target) {
                target.classList.remove("green");
                target.querySelector("svg").classList.remove("filled");

                const count = target.querySelector(".notification");

                if (count) {
                    count.innerText = Number.parseInt(count.innerText) - 1;
                } else {
                    const new_count = document.createElement("span");
                    new_count.className = "notification camo";
                    new_count.innerText = "1";
                    target.appendChild(new_count);
                }
            }

            return $.delete(id);
        }

        if (target) {
            target.classList.add("green");
            target.querySelector("svg").classList.add("filled");

            const count = target.querySelector(".notification");

            if (count) {
                count.innerText = Number.parseInt(count.innerText) + 1;
            } else {
                const new_count = document.createElement("span");
                new_count.className = "notification camo";
                new_count.innerText = "1";
                target.appendChild(new_count);
            }
        }

        return $.create(id, type);
    });
})();
