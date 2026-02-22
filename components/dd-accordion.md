# Accordion component
## Accordion code
Each time an accordion is added, it requires a unique name assigns to the details element. Make this an editable field with the default entry.
**Usage Examples**:
```html
<div class="dd-accordion" data-aos="fade-in" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-accordion__items">
        <details name="[group1]" class="dd-accordion__item">
            <summary class="dd-accordion__header dd-g -y-center">
                <!-- [accordion_title]: required -->
                <div class="dd-accordion__title dd-u-1-1">
                    [accordion_title]
                </div>
                <!-- end [accordion_title]: required -->
            </summary>
            <!-- [accordion_copy]: required -->
            <div class="dd-accordion__copy">
                <p>
                   [accoriod_copy]
                </p>
            </div>
        </details>
        <details name="[group1]" class="dd-accordion__item">
            <summary class="dd-accordion__header dd-g -y-center">
                <!-- [accordion_title]: required -->
                <div class="dd-accordion__title dd-u-1-1">
                    [accordion_title]
                </div>
                <!-- end [accordion_title]: required -->
            </summary>
            <!-- [accordion_copy]: required -->
            <div class="dd-accordion__copy">
                <p>
                   [accoriod_copy]
                </p>
            </div>
        </details>
    </div>
</div>
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
