import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: 'pl3xus',
      url: '/',
    },
    links: [
      {
        text: 'Documentation',
        url: '/docs',
        active: 'nested-url',
      },
      {
        text: 'Blog',
        url: '/blog',
        active: 'nested-url',
      },
      {
        text: 'Showcase',
        url: '/showcase',
        active: 'nested-url',
      },
      {
        text: 'Sponsors',
        url: 'https://www.vertec.io',
        external: true,
      },
    ],
    githubUrl: 'https://github.com/vertec-io/pl3xus',
  };
}
