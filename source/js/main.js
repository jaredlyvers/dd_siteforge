/**
* all document ready functions
**/
document.addEventListener('DOMContentLoaded', () => {

  window.ddSal = sal({
    // Global settings:
    root: null, // IntersectionObserver root; null = viewport
    rootMargin: '0% 0%', // grow/shrink the root bounding box
    threshold: 0.1, // fire when this fraction of the element is visible
    animateClassName: 'sal-animate', // class applied on animation
    disabledClassName: 'sal-disabled', // class applied to body when disabled
    enterEventName: 'sal:in',
    exitEventName: 'sal:out',
    selector: '[data-sal]', // elements to observe
    once: false, // whether animation should happen only once
    // Honor prefers-reduced-motion (WCAG 2.2 2.3.3)
    disabled: window.matchMedia('(prefers-reduced-motion: reduce)').matches,
  });

});

// Re-observe elements injected by HTMX
document.body.addEventListener("htmx:afterSettle", function () {
  if (window.ddSal && typeof window.ddSal.update === 'function') {
    window.ddSal.update();
  }
});
