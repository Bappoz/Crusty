/* Rippler Docs — Navigation JS */

(function () {
  "use strict";

  /* ── Mobile sidebar toggle ── */
  const sidebar  = document.getElementById("sidebar");
  const overlay  = document.getElementById("sidebar-overlay");
  const hamburger = document.getElementById("hamburger");

  function openSidebar() {
    sidebar.classList.add("open");
    overlay.classList.add("active");
    document.body.style.overflow = "hidden";
  }

  function closeSidebar() {
    sidebar.classList.remove("open");
    overlay.classList.remove("active");
    document.body.style.overflow = "";
  }

  if (hamburger) hamburger.addEventListener("click", openSidebar);
  if (overlay)   overlay.addEventListener("click", closeSidebar);

  /* ── Highlight active sub-section in sidebar ── */
  const subLinks = document.querySelectorAll(".sidebar-sub a[href^='#']");

  if (subLinks.length > 0) {
    const sectionIds = Array.from(subLinks).map(a => a.getAttribute("href").slice(1));
    const sections   = sectionIds.map(id => document.getElementById(id)).filter(Boolean);

    function updateActiveSub() {
      let active = null;
      const scrollTop = window.scrollY + 100;
      for (const sec of sections) {
        if (sec.offsetTop <= scrollTop) active = sec.id;
      }
      subLinks.forEach(a => {
        a.classList.toggle("active-sub", a.getAttribute("href") === "#" + active);
      });
    }

    window.addEventListener("scroll", updateActiveSub, { passive: true });
    updateActiveSub();
  }

  /* ── Smooth scroll for same-page anchor links ── */
  document.querySelectorAll('a[href^="#"]').forEach(a => {
    a.addEventListener("click", e => {
      const target = document.querySelector(a.getAttribute("href"));
      if (target) {
        e.preventDefault();
        target.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    });
  });
})();
