import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      redirect: '/sites',
    },
    {
      path: '/sites',
      name: 'sites',
      component: () => import('@/views/SitesView.vue'),
      meta: { title: 'Sites' },
    },
    {
      path: '/networks',
      name: 'networks',
      component: () => import('@/views/NetworksView.vue'),
      meta: { title: 'Networks' },
    },
    {
      path: '/hosts',
      name: 'hosts',
      component: () => import('@/views/HostsView.vue'),
      meta: { title: 'Hosts' },
    },
    {
      path: '/hosts/:id',
      name: 'host-detail',
      component: () => import('@/views/HostDetailView.vue'),
      meta: { title: 'Host Detail' },
      props: (route) => ({ id: Number(route.params.id) }),
    },
    {
      path: '/services',
      name: 'services',
      component: () => import('@/views/ServicesView.vue'),
      meta: { title: 'Services' },
    },
    {
      path: '/addresses',
      name: 'addresses',
      component: () => import('@/views/AddressesView.vue'),
      meta: { title: 'Addresses' },
    },
    {
      path: '/credentials',
      name: 'credentials',
      component: () => import('@/views/CredentialsView.vue'),
      meta: { title: 'Credentials' },
    },
    {
      path: '/tags',
      name: 'tags',
      component: () => import('@/views/TagsView.vue'),
      meta: { title: 'Tags' },
    },
    {
      path: '/notes',
      name: 'notes',
      component: () => import('@/views/NotesView.vue'),
      meta: { title: 'Notes' },
    },
  ],
})

router.afterEach((to) => {
  const title = to.meta?.title as string | undefined
  document.title = title ? `${title} — Network Inventory` : 'Network Inventory'
})

export default router
