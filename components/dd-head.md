---
component: dd-head
version: 1
node_scope: page_head   # special scope: rendered inside <head>, one per page
                        # (site_head is reserved for future global <head> config like analytics, favicon, etc.)

insert:
  defaults:
    title: "Untitled Page"
    meta_description: ""
    canonical_url: ""
    robots: "index, follow"
    schema_type: "WebPage"
    og_title: ""
    og_description: ""
    og_image: ""

fields:
  - id: title
    required: true
    type: string
    maps_to: "<title>"

  - id: meta_description
    required: false
    type: string
    maps_to: "<meta name='description'>"

  - id: canonical_url
    required: false
    type: string
    maps_to: "<link rel='canonical'>"

  - id: robots
    required: false
    type: enum
    options: ["index, follow", "noindex, follow", "index, nofollow", "noindex, nofollow"]
    default: "index, follow"
    maps_to: "<meta name='robots'>"

  - id: schema_type
    required: true
    type: enum
    options: ["WebPage", "Article", "AboutPage", "ContactPage", "CollectionPage", "Organization", "LocalBusiness", "Product", "Service"]
    default: "WebPage"
    maps_to: "<script type='application/ld+json'> @type"

  - id: og_title
    required: false
    type: string
    maps_to: "<meta property='og:title'>"

  - id: og_description
    required: false
    type: string
    maps_to: "<meta property='og:description'>"

  - id: og_image
    required: false
    type: string
    maps_to: "<meta property='og:image'>"

edit_ui:
  tab_order:
    - title
    - meta_description
    - canonical_url
    - robots
    - schema_type
    - og_title
    - og_description
    - og_image

  enter_behavior:
    parent_row: "start head field editing"

  modal_fields:
    parent_edit_modes:
      - title
      - meta_description
      - canonical_url
      - robots
      - schema_type
      - og_title
      - og_description
      - og_image

blueprint:
  label: "dd-head"
  show_fields:
    - title
    - meta_description
    - canonical_url
    - robots
    - schema_type
---

## HTML Template

Rendered inside `<head>` of every page. Chrome tags (charset, viewport,
stylesheet link) are hardcoded and always present; the user-configurable
fields render conditionally below them.

```html
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>[title]</title>
  <!-- if meta_description -->
  <meta name="description" content="[meta_description]">
  <!-- endif -->
  <!-- if canonical_url -->
  <link rel="canonical" href="[canonical_url]">
  <!-- endif -->
  <meta name="robots" content="[robots]">
  <!-- if og_title -->
  <meta property="og:title" content="[og_title]">
  <!-- endif -->
  <!-- if og_description -->
  <meta property="og:description" content="[og_description]">
  <!-- endif -->
  <!-- if og_image -->
  <meta property="og:image" content="[og_image]">
  <!-- endif -->
  <link rel="apple-touch-icon" sizes="180x180" href="/assets/favicon/apple-touch-icon.png" />
  <link rel="icon" type="image/png" sizes="32x32" href="/assets/favicon/favicon-32x32.png" />
  <link rel="icon" type="image/png" sizes="16x16" href="/assets/favicon/favicon-16x16.png" />
  <link rel="manifest" href="/assets/favicon/site.webmanifest" />
  <link rel="mask-icon" href="/assets/favicon/safari-pinned-tab.svg" color="#5bbad5" />
  <link rel="shortcut icon" href="/assets/favicon/favicon.ico" />
  <meta name="msapplication-TileColor" content="#ffffff" />
  <meta name="msapplication-config" content="/assets/favicon/browserconfig.xml" />
  <meta name="theme-color" content="#ffffff" />
  <link rel="stylesheet" href="/assets/css/style.min.css">
  <script type="application/ld+json">
  {
    "@context": "https://schema.org",
    "@type": "[schema_type]",
    "name": "[title]"
    <!-- if meta_description -->,
    "description": "[meta_description]"
    <!-- endif -->
    <!-- if canonical_url -->,
    "url": "[canonical_url]"
    <!-- endif -->
    <!-- if og_image -->,
    "image": "[og_image]"
    <!-- endif -->
  }
  </script>
</head>
```

## Conditional Markup

- `<title>` always renders (field is required)
- `<meta name="description">` renders only when `meta_description` is non-empty
- `<link rel="canonical">` renders only when `canonical_url` is non-empty
- `<meta name="robots">` always renders (defaults to `"index, follow"`)
- Each OpenGraph meta (`og:title`, `og:description`, `og:image`) renders independently when its field is non-empty
- favicon + manifest + theme-color chrome tags always render
- `<script type="application/ld+json">` always renders; `@type` comes from `schema_type`; optional properties (`description`, `url`, `image`) are included only when their source fields are non-empty
- chrome tags (`charset`, `viewport`, stylesheet link) always render

## Validation Rules

- `title` required and non-empty (after trimming)
- `meta_description` optional; recommended 50–160 characters when provided (warning only, not a hard fail)
- `canonical_url` optional; when provided, must pass URL check (`http://`, `https://`, or `/`)
- `robots` optional; must be one of the enum options when provided
- `schema_type` required; must be one of the enum options (`WebPage`, `Article`, `AboutPage`, `ContactPage`, `CollectionPage`, `Organization`, `LocalBusiness`, `Product`, `Service`)
- `og_title`, `og_description`, `og_image` all optional and independent; `og_image` must pass URL check when provided
- rendered LD-JSON must be valid JSON — renderer escapes string values and omits optional keys with trailing-comma safety
