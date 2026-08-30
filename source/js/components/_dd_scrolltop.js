function dd_scrolltop() {
  let scrollTO = null;
  let scrollTop = 0;

  window.addEventListener('scroll', () => {
    if (scrollTO) clearTimeout(scrollTO);
    scrollTO = setTimeout(() => {
      scrollTop = window.scrollY;

      const btn = document.querySelector('.dd-scrolltop');
      if (!btn) return;

      if (scrollTop > 5) {
        btn.classList.add('-active');
      } else {
        btn.classList.remove('-active');
      }
    }, 0);
  });
};

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_scrolltop();
});
