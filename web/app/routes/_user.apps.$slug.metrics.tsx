import { useParams } from 'react-router';
import { AppMetricsView } from '~/components/apps/metrics';

export default function AppMetricsPage() {
  const { slug } = useParams();
  return <AppMetricsView appSlug={slug!} />;
}
