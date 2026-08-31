# CL8Y-web blog drop-in contract

Worker output `post.mdx` must satisfy `CL8Y-web` `src/blog/blogIndex.ts`:

Required authored frontmatter:

- `title`, `description`, `slug`, `date`, `author`, `image`, `tags`

`wordCount` is computed at **site build** from the MDX body. The worker must not author it.

Hero file:

- Worker path: `public/images/blog/<slug>-hero.jpg` (after copy)
- Frontmatter `image: /images/blog/<slug>-hero.jpg`
- Do not put rasters in `src/blog/assets`

Human publish:

1. Copy `post.mdx` → `CL8Y-web/src/blog/posts/<slug>.mdx`
2. Copy hero → `CL8Y-web/public/images/blog/<slug>-hero.jpg`
3. Open a feature-branch MR. Do not push `main`.
4. `yarn test && yarn typecheck && yarn lint && yarn build`
5. Confirm `/blog/<slug>` prerendered and `dist/rss.xml` contains the item.

Voice: `CL8Y-web/blog_gen/SKILL.md`.
