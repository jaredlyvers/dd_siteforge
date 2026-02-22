---
component: dd-hero
version: 1
node_scope: page_node
insert:
  defaults:
    hero_class: "-full-full"
    hero_data_aos: "fade-in"
    image: "https://dummyimage.com/1920x1080/000/fff"
    title: "Build with dd-framework"
    subtitle: "Framework-native static page builder"
    copy: "Compose pages with typed component schemas."
    link_1_text: "Get Started"
    link_1_url: "/start"
    link_2_text: "Learn More"
    link_2_url: "/learn-more"
fields:
  - id: image
    required: true
    type: string
    maps_to: ".dd-hero__image img[src]"
  - id: hero_class
    required: true
    type: enum
    options: ["-contained", "-contained-md", "-contained-lg", "-contained-xl", "-contained-xxl", "-full-full", "-full-contained", "-full-contained-md", "-full-contained-lg", "-full-contained-xl", "-full-contained-xxl"]
    default: "-full-full"
    maps_to: ".dd-hero class token"
  - id: hero_data_aos
    required: true
    type: enum
    options: ["fade-in", "fade-up", "fade-right", "fade-down", "fade-left", "zoom-in", "zoom-in-up", "zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-hero__content[data-aos]"
  - id: title
    required: true
    type: string
    maps_to: ".dd-hero__title h1"
  - id: subtitle
    required: false
    type: string
    maps_to: ".dd-hero__subtitle"
  - id: copy
    required: false
    type: string
    maps_to: ".dd-hero__body p"
  - id: link_1_text
    required: false
    type: string
    maps_to: ".dd-hero__links .dd-hero__link:first-child a"
  - id: link_1_url
    required: false
    type: string
    maps_to: ".dd-hero__links .dd-hero__link:first-child a[href]"
  - id: link_2_text
    required: false
    type: string
    maps_to: ".dd-hero__links .dd-hero__link:nth-child(2) a"
  - id: link_2_url
    required: false
    type: string
    maps_to: ".dd-hero__links .dd-hero__link:nth-child(2) a[href]"
edit_ui:
  tab_order:
    - image
    - hero_class
    - hero_data_aos
    - title
    - subtitle
    - copy
    - link_1_text
    - link_1_url
    - link_2_text
    - link_2_url
blueprint:
  label: "dd-hero"
  show_fields:
    - hero_class
    - hero_data_aos
    - title
    - subtitle
    - link_1_text
    - link_1_url
    - link_2_text
    - link_2_url
    - image
---

## HTML Template

```html
<section class="dd-hero [hero_class]" aria-label="Introduction">
  <div class="dd-hero__image">
    <picture>
      <img src="[image]" class="dd-img" alt="[image_alt]" />
    </picture>
  </div>
  <div class="dd-hero__content dd-g" data-aos="[hero_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-hero__copy dd-u-1-1 dd-u-lg-12-24">
      <div class="dd-hero__title"><h1>[title]</h1></div>
      <!-- if [subtitle] --><div class="dd-hero__subtitle"><strong>[subtitle]</strong></div><!-- endif -->
      <!-- if [copy] --><div class="dd-hero__body"><p>[copy]</p></div><!-- endif -->
      <!-- if [link_1_text] --><div class="dd-hero__links dd-g">
        <div class="dd-hero__link"><a href="[link_1_url]" class="dd-button -primary">[link_1_text]</a></div>
        <!-- if [link_2_text] --><div class="dd-hero__link"><a href="[link_2_url]" class="dd-button -ghost">[link_2_text]</a></div><!-- endif -->
      </div><!-- endif -->
    </div>
  </div>
</section>
```
