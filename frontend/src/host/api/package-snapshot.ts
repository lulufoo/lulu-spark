import { invoke } from './transport.ts';

export type PackageSnapshot = {
  is_debug: boolean;
  version: string;
  product_name: string;
};

export async function getPackageSnapshot(): Promise<PackageSnapshot> {
  return invoke('get_package_snapshot') as Promise<PackageSnapshot>;
}
