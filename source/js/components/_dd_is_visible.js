function dd_is_visible() {
  const animate_observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add('-is-visible');
        }
        else {
          entry.target.classList.remove('-is-visible');
        }
      });
    });
    // Only check for elements with a class of -animate
    const animate_elements = document.querySelectorAll('.-animate');
    // watch and swap classes
    animate_elements.forEach((el) => animate_observer.observe(el));
}

// Initialize on initial page load
document.addEventListener('DOMContentLoaded', () => {
  dd_is_visible();
});
