import { fetch } from 'node-fetch';

export async function fetchData(url) {
  const response = await fetch(url);
  return response.json();
}
