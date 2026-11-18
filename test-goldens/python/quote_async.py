async def fetch_data(url: str) -> bytes:
    async with aiohttp.ClientSession() as session:
        response = await session.get(url)
        return await response.read()
