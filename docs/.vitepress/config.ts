import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(
  defineConfig({
    title: 'myenv',
    description: 'rhi ecosystem orchestrator',

    base: '/myenv/',

    themeConfig: {
      nav: [
        { text: 'Guide', link: '/introduction' },
        { text: 'Manifest', link: '/manifest' },
        { text: 'Seeds', link: '/seeds' },
        { text: 'rhi', link: 'https://rhizome-lab.github.io/' },
      ],

      sidebar: [
        {
          text: 'Guide',
          items: [
            { text: 'Introduction', link: '/introduction' },
            { text: 'Getting Started', link: '/getting-started' },
            { text: 'Design', link: '/design' },
          ]
        },
        {
          text: 'Reference',
          items: [
            { text: 'Manifest Format', link: '/manifest' },
            { text: 'Seeds', link: '/seeds' },
            { text: 'Tool Integration', link: '/tool-integration' },
          ]
        },
      ],

      socialLinks: [
        { icon: 'github', link: 'https://github.com/rhizome-lab/myenv' }
      ],

      search: {
        provider: 'local'
      },

      editLink: {
        pattern: 'https://github.com/rhizome-lab/myenv/edit/master/docs/:path',
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
