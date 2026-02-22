# Hero component
## Hero code
**Usage Examples**:
```html
<section class="dd-hero" id="dd-0001" aria-label="Introduction">
  <!-- if [hero_image]: mobile and desktop images -->
  <div class="dd-hero__image">
    <picture>
      <source srcset="
      https://dummyimage.com/1024x505/000000/fff 720w,
      https://dummyimage.com/1920x1080/000000/fff 1440w"
      sizes="(max-width: 1440px) 100vw, 1440px">
      <img src="https://dummyimage.com/1920x1080/000000/fff" class="dd-img" alt="Hero image description" />
    </picture>
  </div>
  <style>
    // mobile image
    .dd-hero__image {
      background-image: url('https://dummyimage.com/720x405/000000/fff');
    }
    // desktop image
    @media only screen and (min-width: 64em) {
      .dd-hero__image {
        background-image: url('https://dummyimage.com/1920x1080/000000/fff');
      }
    }
  </style>
  <!-- endif -->
  <div class="dd-hero__content dd-g" data-aos="fade-in" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-hero__copy dd-u-1-1 dd-u-lg-12-24">
      <!-- [hero__title]: required -->
      <div class="dd-hero__title">
        <h1>[hero_title]</h1>
      </div>
      <!-- end [hero__title]: required -->
      <!-- if [hero_subtitle] -->
      <div class="dd-hero__subtitle">
        <strong>[hero_subtitle]</strong>
      </div>
      <!-- endif -->
      <!-- if [hero_body] -->
      <div class="dd-hero__body">
        <p>Hero Body Copy</p>
        <!-- if [hero_link] -->
        <div class="dd-hero__links dd-g">
          <div class="dd-hero__link">
            <a href="#" class="dd-button -primary">[hero_link]</a>
          </div>
          <div class="dd-hero__link">
            <a href="#" class="dd-button -ghost">[hero_link_2]</a>
          </div>
        </div>
        <!-- endif -->
      </div>
      <!-- endif -->
    </div>
  </div>
</section>
```html

## CSS classes
**Usage Examples**:
```css
.dd-hero__image.-contained
.dd-hero__image.-contained-md
.dd-hero__image.-contained-lg
.dd-hero__image.-contained-xl
.dd-hero__image.-contained-xxl
.dd-hero__image.-full-full // default class
.dd-hero__image.-full-contained
.dd-hero__image.-full-contained-md
.dd-hero__image.-full-contained-lg
.dd-hero__image.-full-contained-xl
.dd-hero__image.-full-contained-xxl
```css

## data-aos options
**Usage Examples**: .dd-hero__content element only
```text
fade-in
fade-up
fade-right
fade-down
fade-left
zoom-in
zoom-in-up
zoom-in-down
```
