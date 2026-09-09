import { redirect } from 'react-router';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  throw redirect(`/apps/${params.slug}/deployments`);
}

export default function AppIndexPage() {
  return null;
}
