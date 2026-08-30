function dd_slider() {
  const sliders = document.querySelectorAll('.dd-slider');

  if (!sliders.length) {
    return;
  }

  sliders.forEach((sliderContainer) => {
    const slides = sliderContainer.querySelectorAll('.dd-slider__item');
    const prevBtn = sliderContainer.querySelector('.dd-slider__previous');
    const nextBtn = sliderContainer.querySelector('.dd-slider__next');
    const tabsContainer = sliderContainer.querySelector('.dd-slider__tabs');

    if (!slides.length) {
      return;
    }

    let currentSlide = 0;
    const sliderId = sliderContainer.dataset.sliderId || Math.random().toString(36).substr(2, 9);
    sliderContainer.dataset.sliderId = sliderId;

    function createTabs() {
      if (!tabsContainer) return;
      tabsContainer.innerHTML = '';
      slides.forEach((_, i) => {
        const li = document.createElement('li');
        const button = document.createElement('button');
        button.textContent = i + 1;
        button.addEventListener('click', () => {
          currentSlide = i;
          showSlide(currentSlide);
        });
        li.appendChild(button);
        tabsContainer.appendChild(li);
      });
    }

    function updateTabs() {
      if (!tabsContainer) return;
      const tabListButton = tabsContainer.querySelectorAll('button');
      tabListButton.forEach((btn, i) => {
        btn.classList.toggle('-current', i === currentSlide);
      });
    }

    function showSlide(index) {
      const position = (0 - index) * 100;
      slides.forEach((slide, i) => {
        slide.style.left = `${position}%`;
        slide.classList.toggle('-current', i === index);
      });
      updateTabs();
    }

    function nextSlide() {
      currentSlide = (currentSlide + 1) % slides.length;
      showSlide(currentSlide);
    }

    function prevSlide() {
      currentSlide = (currentSlide - 1 + slides.length) % slides.length;
      showSlide(currentSlide);
    }

    createTabs();
    updateTabs();

    if (nextBtn) {
      nextBtn.addEventListener('click', nextSlide);
    }
    if (prevBtn) {
      prevBtn.addEventListener('click', prevSlide);
    }

    sliderContainer.addEventListener('keydown', (e) => {
      if (e.key === 'ArrowLeft') {
        prevSlide();
      } else if (e.key === 'ArrowRight') {
        nextSlide();
      }
    });
  });
}

document.addEventListener('DOMContentLoaded', () => {
  dd_slider();
});

document.body.addEventListener("htmx:afterSettle", function (event) {
  dd_slider();
});
