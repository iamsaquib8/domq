// Language tabs for the guide.
//
// A tab group is a <div class="domq-tabs"> whose children are a bold label
// paragraph (**Rust**) followed by the blocks that belong to that label.
// Without this script, or without JavaScript, the page shows every label and
// every block one after the other. With it, one language shows at a time and
// the choice is remembered across pages.
(function () {
  "use strict";

  var KEY = "domq-lang";
  var groups = [];
  var counter = 0;

  function read() {
    try { return localStorage.getItem(KEY); } catch (e) { return null; }
  }

  function write(value) {
    try { localStorage.setItem(KEY, value); } catch (e) { /* private mode */ }
  }

  function isLabel(el) {
    var only = el.firstElementChild;
    return el.tagName === "P" && only && only.tagName === "STRONG" &&
      el.children.length === 1 &&
      el.textContent.trim() === only.textContent.trim();
  }

  function build(container) {
    var tabs = [];
    var current = null;
    Array.prototype.slice.call(container.children).forEach(function (el) {
      if (isLabel(el)) {
        current = { label: el.textContent.trim(), nodes: [] };
        tabs.push(current);
        container.removeChild(el);
      } else if (current) {
        current.nodes.push(el);
      }
    });
    if (tabs.length < 2) { return; }

    var id = "domq-tabs-" + (counter += 1);
    var list = document.createElement("div");
    list.className = "domq-tablist";
    list.setAttribute("role", "tablist");

    var group = { tabs: [] };
    tabs.forEach(function (tab, i) {
      var button = document.createElement("button");
      button.type = "button";
      button.className = "domq-tab";
      button.id = id + "-tab-" + i;
      button.setAttribute("role", "tab");
      button.setAttribute("aria-controls", id + "-panel-" + i);
      button.textContent = tab.label;

      var panel = document.createElement("div");
      panel.className = "domq-panel";
      panel.id = id + "-panel-" + i;
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", button.id);
      panel.setAttribute("data-label", tab.label);
      tab.nodes.forEach(function (node) { panel.appendChild(node); });

      container.appendChild(panel);
      list.appendChild(button);
      group.tabs.push({ key: tab.label.toLowerCase(), button: button, panel: panel });

      button.addEventListener("click", function () { choose(tab.label.toLowerCase(), true); });
      button.addEventListener("keydown", function (event) {
        var n = group.tabs.length;
        var to = null;
        if (event.key === "ArrowRight") { to = (i + 1) % n; }
        else if (event.key === "ArrowLeft") { to = (i + n - 1) % n; }
        else if (event.key === "Home") { to = 0; }
        else if (event.key === "End") { to = n - 1; }
        if (to !== null) {
          event.preventDefault();
          choose(group.tabs[to].key, true);
          group.tabs[to].button.focus();
        }
      });
    });

    container.insertBefore(list, container.firstChild);
    container.classList.add("domq-tabs-ready");
    show(group, group.tabs[0].key);
    groups.push(group);
  }

  function show(group, key) {
    var found = group.tabs.some(function (tab) { return tab.key === key; });
    if (!found) { return; }
    group.tabs.forEach(function (tab) {
      var on = tab.key === key;
      tab.button.setAttribute("aria-selected", on ? "true" : "false");
      tab.button.tabIndex = on ? 0 : -1;
      tab.panel.hidden = !on;
    });
  }

  // Show `key` in every group that has it; groups without it keep their tab.
  function choose(key, remember) {
    groups.forEach(function (group) { show(group, key); });
    if (remember) { write(key); }
  }

  function init() {
    Array.prototype.forEach.call(document.querySelectorAll(".domq-tabs"), build);
    var saved = read();
    if (saved) { choose(saved, false); }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
