// Search box: filters SEARCH_INDEX (pages and functions, written by
// generate.py above this script) as you type. Arrow keys move, Enter opens.
(function () {
  const input = document.getElementById("search");
  const list = document.getElementById("search-results");
  if (!input || !list) return;
  let selected = -1;

  function score(entry, query) {
    const title = entry.title.toLowerCase();
    if (title === query) return 0;
    if (title.startsWith(query)) return 1;
    if (title.includes(query)) return 2;
    if (entry.kind.toLowerCase().includes(query)) return 3;
    return -1;
  }

  function render() {
    const query = input.value.trim().toLowerCase();
    list.innerHTML = "";
    selected = -1;
    if (!query) {
      list.hidden = true;
      return;
    }
    const matches = SEARCH_INDEX.map((entry) => [score(entry, query), entry])
      .filter(([rank]) => rank >= 0)
      .sort((a, b) => a[0] - b[0] || a[1].title.localeCompare(b[1].title))
      .slice(0, 12);
    for (const [, entry] of matches) {
      const item = document.createElement("li");
      const link = document.createElement("a");
      link.href = entry.url;
      link.textContent = entry.title;
      const kind = document.createElement("span");
      kind.className = "kind";
      kind.textContent = entry.kind;
      link.appendChild(kind);
      item.appendChild(link);
      list.appendChild(item);
    }
    if (!matches.length) {
      const item = document.createElement("li");
      item.innerHTML = '<a>No matches</a>';
      list.appendChild(item);
    }
    list.hidden = false;
  }

  function move(step) {
    const links = list.querySelectorAll("a[href]");
    if (!links.length) return;
    if (selected >= 0) links[selected].classList.remove("selected");
    selected = (selected + step + links.length) % links.length;
    links[selected].classList.add("selected");
  }

  input.addEventListener("input", render);
  input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown") { move(1); event.preventDefault(); }
    else if (event.key === "ArrowUp") { move(-1); event.preventDefault(); }
    else if (event.key === "Enter") {
      const links = list.querySelectorAll("a[href]");
      const target = links[selected >= 0 ? selected : 0];
      if (target) window.location = target.href;
    } else if (event.key === "Escape") {
      input.value = "";
      render();
    }
  });
  document.addEventListener("click", (event) => {
    if (!event.target.closest(".search")) list.hidden = true;
  });
  // "/" focuses the search box, as on many documentation sites.
  document.addEventListener("keydown", (event) => {
    if (event.key === "/" && document.activeElement !== input) {
      event.preventDefault();
      input.focus();
    }
  });
})();
