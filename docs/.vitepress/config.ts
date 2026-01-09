import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(
  defineConfig({
    title: 'Nursery',
    description: 'Rhizome ecosystem orchestrator',

    base: '/nursery/',

    themeConfig: {
      nav: [
        { text: 'Guide', link: '/introduction' },
        { text: 'Manifest', link: '/manifest' },
        { text: 'Seeds', link: '/seeds' },
        { text: 'Rhizome', link: 'https://rhizome-lab.github.io/' },
      ],

      sidebar: [
        {
          text: 'Guide',
          items: [
            { text: 'Introduction', link: '/introduction' },
            { text: 'Getting Started', link: '/getting-started' },
          ]
        },
        {
          text: 'Reference',
          items: [
            { text: 'Manifest Format', link: '/manifest' },
            { text: 'Seeds', link: '/seeds' },
          ]
        },
      ],

      socialLinks: [
        { icon: 'github', link: 'https://github.com/rhizome-lab/nursery' }
      ],

      search: {
        provider: 'local'
      },

      editLink: {
        pattern: 'https://github.com/rhizome-lab/nursery/edit/master/docs/:path',
        text: 'Edit this page on GitHub'
      },
    },

    vite: {
      optimizeDeps: {
        include: ['mermaid'],
      },
    },
  }),
)
