//! Namespace Loader
//! https://github.com/hkauso/regns

globalThis.ns_config = globalThis.ns_config || {
    root: "/static/js/",
    version: 0,
    verbose: true,
};

globalThis._app_base = globalThis._app_base || { ns_store: {}, classes: {} };

function regns_log(level, ...args) {
    if (globalThis.ns_config.verbose) {
        console[level](...args);
    } else {
        return;
    }
}

/// Query an existing namespace
globalThis.ns = (ns) => {
    regns_log("info", "namespace query:", ns);

    // get namespace from app base
    const res = globalThis._app_base.ns_store[`$${ns}`];

    if (!res) {
        return console.error(
            "namespace does not exist, please use one of the following:",
            Object.keys(globalThis._app_base.ns_store),
        );
    }

    return res;
};

/// Register a new namespace
globalThis.reg_ns = (ns, deps) => {
    if (typeof ns !== "string") {
        return console.error("type check failed on namespace:", ns);
    }

    if (!ns) {
        return console.error("cannot register invalid namespace!");
    }

    if (globalThis._app_base.ns_store[`$${ns}`]) {
        regns_log("warn", "overwriting existing namespace:", ns);
    }

    // register new blank namespace
    globalThis._app_base.ns_store[`$${ns}`] = {
        _ident: ns,
        _deps: deps || [],
        /// Pull dependencies (other namespaces) as listed in the given `deps` argument
        _get_deps: () => {
            const self = globalThis._app_base.ns_store[`$${ns}`];
            const deps = {};

            for (const dep of self._deps) {
                const res = globalThis.ns(dep);

                if (!res) {
                    regns_log("warn", "failed to pull dependency:", dep);
                    continue;
                }

                deps[dep] = res;
            }

            deps.$ = self; // give access to self through $
            return deps;
        },
        /// Store the real versions of functions
        _fn_store: {},
        /// Call a function in a namespace and load namespace dependencies
        define: (name, func, types) => {
            const self = globalThis.ns(ns);
            self._fn_store[name] = func; // store real function
            self[name] = function (...args) {
                regns_log("info", "namespace call:", ns, name);

                // js doesn't provide type checking, we do
                if (types) {
                    for (const i in args) {
                        // biome-ignore lint: this is incorrect, you do not need a string literal to use typeof
                        if (types[i] && typeof args[i] !== types[i]) {
                            return console.error(
                                "argument does not pass type check:",
                                i,
                                args[i],
                            );
                        }
                    }
                }

                // ...
                // we MUST return here, otherwise nothing will work in workers
                return self._fn_store[name](self._get_deps(), ...args); // call with deps and arguments
            };
        },
    };

    regns_log("log", "registered namespace:", ns);
    return globalThis._app_base.ns_store[`$${ns}`];
};

/// Call a namespace function quickly
globalThis.trigger = (id, args) => {
    // get namespace
    const [namespace, func] = id.split(":");
    const self = ns(namespace);

    if (!self) {
        return console.error("namespace does not exist:", namespace);
    }

    if (!self[func]) {
        return console.error("namespace function does not exist:", id);
    }

    return self[func](...(args || []));
};

/// Import a namespace from path (relative to ns_config.root)
globalThis.use = (id, callback) => {
    // check if namespace already exists
    const res = globalThis._app_base.ns_store[`$${id}`];

    if (res) {
        return callback(res);
    }

    // create script to load
    const script = document.createElement("script");
    script.src = `${globalThis.ns_config.root}${id}.js?v=${globalThis.ns_config.version}`;
    script.id = `${globalThis.ns_config.version}-${id}.js`;
    document.head.appendChild(script);

    script.setAttribute("data-turbo-permanent", "true");
    script.setAttribute("data-registered", new Date().toISOString());
    script.setAttribute("data-version", globalThis.ns_config.version);

    // run callback once the script loads
    script.addEventListener("load", () => {
        const res = globalThis._app_base.ns_store[`$${id}`];

        if (!res) {
            return console.error("imported namespace failed to register:", id);
        }

        callback(res);
    });
};

// classes

/// Import a class from path (relative to ns_config.root/classes)
globalThis.require = (id, callback) => {
    // check if class already exists
    const res = globalThis._app_base.classes[id];

    if (res) {
        return callback(res);
    }

    // create script to load
    const script = document.createElement("script");
    script.src = `${globalThis.ns_config.root}classes/${id}.js?v=${globalThis.ns_config.version}`;
    script.id = `${globalThis.ns_config.version}-${id}.class.js`;
    document.head.appendChild(script);

    script.setAttribute("data-turbo-permanent", "true");
    script.setAttribute("data-registered", new Date().toISOString());
    script.setAttribute("data-version", globalThis.ns_config.version);

    // run callback once the script loads
    script.addEventListener("load", () => {
        const res = globalThis._app_base.classes[id];

        if (!res) {
            return console.error("imported class failed to register:", id);
        }

        callback(res);
    });
};

globalThis.define = (class_name, class_) => {
    globalThis._app_base.classes[class_name] = class_;
    regns_log("log", "registered class:", class_name);
};
