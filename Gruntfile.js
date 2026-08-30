module.exports = (grunt) => {
  // init configs
  grunt.initConfig({
    pkg: grunt.file.readJSON('package.json'),
    concat: {
      options: {
        seperator: ';',
        compress: true
      },
      dist: {
        src: [
          'source/js/vendors/**/*.js',
          'source/js/components/*.js',
          'source/js/main.js',
        ],
        dest: 'web/assets/js/main.js'
      }
    },
    uglify: {
      options: {
        seperator: ';',
        compress: true
      },
      dist: {
        src: [
          'source/js/vendors/**/*.js',
          'source/js/components/*.js',
          'source/js/main.js',
        ],
        dest: 'web/assets/js/main.min.js'
      }
    },
    'dart-sass': {
      sassmin: {
        options: {
          outputStyle: 'compressed',
          sourceMap: true
        },
        files: {
          'web/assets/css/style.min.css': 'source/scss/style.scss'
        }
      },
      sass: {
        options: {
          outputStyle: 'expanded',
          sourceMap: true
        },
        files: {
          'web/assets/css/style.css': 'source/scss/style.scss'
        }
      },

    },
    realFavicon: {
      favicons: {
        src: 'source/favicon/favicon.png',
        dest: 'web/assets/favicon',
        options: {
          iconsPath: '/assets/favicon/',
          html: ['web/includes/favicon_data.json'],
          design: {
            ios: {
              pictureAspect: 'backgroundAndMargin',
              backgroundColor: '#ffffff',
              margin: '14%',
              assets: {
                ios6AndPriorIcons: false,
                ios7AndLaterIcons: false,
                precomposedIcons: false,
                declareOnlyDefaultIcon: true
              }
            },
            desktopBrowser: {
              design: 'raw'
            },
            windows: {
              pictureAspect: 'noChange',
              backgroundColor: '#ffffff',
              onConflict: 'override',
              assets: {
                windows80Ie10Tile: false,
                windows10Ie11EdgeTiles: {
                  small: false,
                  medium: true,
                  big: false,
                  rectangle: false
                }
              }
            },
            androidChrome: {
              pictureAspect: 'backgroundAndMargin',
              margin: '17%',
              backgroundColor: '#ffffff',
              themeColor: '#ffffff',
              manifest: {
                display: 'standalone',
                orientation: 'notSet',
                onConflict: 'override',
                declared: true
              },
              assets: {
                legacyIcon: false,
                lowResolutionIcons: false
              }
            },
            safariPinnedTab: {
              pictureAspect: 'blackAndWhite',
              threshold: 64.0625,
              themeColor: '#5bbad5'
            }
          },
          settings: {
            scalingAlgorithm: 'Mitchell',
            errorOnImageTooSmall: false,
            readmeFile: false,
            htmlCodeFile: false,
            usePathAsIs: false
          }
        }
      }
    },
    replace: {
      faviconStripTags: {
        src: ['source/favicon/favicon_data.json'],
        dest: 'source/templates/favicon.hbs',
        replacements: [
          {
            from: /<html>|<\/html>|<head>|<\/head>|<body>|<\/body>/ig,
            to: ''
          }
        ]
      }
    },
    copy: {
      favicon: {
        nonull: true,
        src: 'web/assets/favicon/favicon.ico',
        dest: 'web/favicon.ico',
      },
      axe: {
        nonull: true,
        src: 'node_modules/axe-core/axe.js',
        dest: 'web/assets/vendors/axe/axe.js',
      },
      axemin: {
        nonull: true,
        src: 'node_modules/axe-core/axe.min.js',
        dest: 'web/assets/vendors/axe/axe.min.js',
      },
      nineaxe: {
        nonull: true,
        src: 'source/vendors/axe/dd-axe.js',
        dest: 'web/assets/vendors/axe/dd-axe.js',
      },
      webfonts: {
        expand: true,
        cwd: 'source/webfonts',
        src: ['**/*'],
        dest: 'web/assets/webfonts/',
      },
      faviconAssets: {
        expand: true,
        cwd: 'source/favicon',
        src: ['**/*'],
        dest: 'web/assets/favicon/',
      },
    },
    clean: {
      cssjs: [
        'web/assets/css',
        'web/assets/js'
      ],
      webfonts: [
        'web/assets/webfonts'
      ],
      favicon: [
        'web/assets/favicon',
        'web/includes/favicon_data.json',
        'web/includes/favicon.php',
        'web/favicon.ico'
      ],
      axe: [
        'web/assets/vendors/axe/'
      ]
    },
    watch: {
      css: {
        files: 'source/**/*.scss',
        tasks: ['dart-sass:sass', 'dart-sass:sassmin']
      },
      js: {
        files: 'source/**/*.js',
        tasks: ['concat', 'uglify']
      },
      axecli: {
        files: 'source/vendors/axe/*.js',
        tasks: ['copy:nineaxe']
      }
    },
    browserSync: {
      dev: {
        bsFiles: {
          src: [
            'web/assets/css/*.css',
            'web/assets/js/*.js',
            'web/**/*.html'
          ]
        },
        options: (function () {
          const proxy = process.env.BROWSERSYNC_PROXY;
          const opts = {
            open: false,
            watchTask: true,
          };
          if (proxy) {
            opts.proxy = { target: proxy };
            if (proxy.indexOf('https://') === 0) {
              opts.https = true;
            }
          } else {
            opts.server = { baseDir: 'web' };
          }
          return opts;
        })(),
      }
    }
  });

  // Load tasks
  grunt.loadNpmTasks('grunt-contrib-concat');
  grunt.loadNpmTasks('grunt-contrib-watch');
  grunt.loadNpmTasks('grunt-express-server');
  grunt.loadNpmTasks('grunt-dart-sass');
  grunt.loadNpmTasks('grunt-contrib-clean');
  grunt.loadNpmTasks('grunt-contrib-uglify');
  grunt.loadNpmTasks('grunt-real-favicon');
  grunt.loadNpmTasks('grunt-text-replace');
  grunt.loadNpmTasks('grunt-notify');
  grunt.loadNpmTasks('grunt-browser-sync');
  grunt.loadNpmTasks('grunt-contrib-copy');

  // Register tasks
  grunt.registerTask('build', [
    'clean:cssjs',
    'clean:webfonts',
    'clean:axe',
    'copy:axe',
    'copy:axemin',
    'copy:nineaxe',
    'copy:webfonts',
    'copy:faviconAssets',
    'dart-sass:sass',
    'dart-sass:sassmin',
    'uglify',
    'concat'
  ]);
  grunt.registerTask('dev', [
    'clean:cssjs',
    'clean:webfonts',
    'clean:axe',
    'copy:axe',
    'copy:axemin',
    'copy:nineaxe',
    'copy:webfonts',
    'copy:faviconAssets',
    'dart-sass:sass',
    'dart-sass:sassmin',
    'uglify',
    'concat',
    'browserSync',
    'watch'
  ]);

  grunt.registerTask('sync', [
    'browserSync',
    'watch'
  ]);
  grunt.registerTask('favicon', [
    'clean:favicon',
    'realFavicon',
    'replace:faviconStripTags',
    'copy:favicon'
  ]);
};
