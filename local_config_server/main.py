import asyncio
from fastapi import FastAPI
import os
import aiohttp


DATA_STORE: dict[str, int | None] = {
    "BUS0": None,
    "BUS1": None,
    "BUS2": None,
    "BUS3": None,
    "BUS4": None,
    "BUS5": None,
    "BUS6": None,
    "BUS7": None,
}
HELIUS_URL = os.environ["HELIUS_URL"]
BUS_DICT = {
    "BUS0": "9ShaCzHhQNvH8PLfGyrJbB8MeKHrDnuPMLnUDLJ2yMvz",
    "BUS1": "4Cq8685h9GwsaD5ppPsrtfcsk3fum8f9UP4SPpKSbj2B",
    "BUS2": "8L1vdGdvU3cPj9tsjJrKVUoBeXYvAzJYhExjTYHZT7h7",
    "BUS3": "JBdVURCrUiHp4kr7srYtXbB7B4CwurUt1Bfxrxw6EoRY",
    "BUS4": "DkmVBWJ4CLKb3pPHoSwYC2wRZXKKXLD2Ued5cGNpkWmr",
    "BUS5": "9uLpj2ZCMqN6Yo1vV6yTkP6dDiTTXmeM5K3915q5CHyh",
    "BUS6": "EpcfjBs8eQ4unSMdowxyTE8K3vVJ3XUnEr5BEWvSX7RB",
    "BUS7": "Ay5N9vKS2Tyo2M9u9TFt59N1XbxdW93C7UrFZW3h8sMC",
}

_background_tasks = set()

app = FastAPI()

@app.on_event("startup")
async def startup_event():
    loop = asyncio.get_running_loop()

    async def periodic_update():
        while True:
            await update_data_store()
            print("Updated data store")
            await asyncio.sleep(10)

    task = loop.create_task(periodic_update())
    _background_tasks.add(task)
    task.add_done_callback(_background_tasks.discard)


async def update_data_store():
    async def make_request(session: aiohttp.ClientSession, key) -> dict | None:
        body = {
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getPriorityFeeEstimate",
            "params": [
                {
                    "accountKeys": [key],
                    "options": {"includeAllPriorityFeeLevels": True}
                },
            ],
        }
        try:
            async with session.post(HELIUS_URL, json=body, timeout=5) as response:
                response.raise_for_status()
                return await response.json()
        except Exception as e:
            print(f"Error making request to helius: {e}")
            return None


    async with aiohttp.ClientSession() as session:
        responses = await asyncio.gather(*[
            make_request(session, key) for bus, key in BUS_DICT.items()
        ])


    for (bus, key), response_json in zip(BUS_DICT.items(), responses):
        if not response_json:
            print(f"Error updating fee for {bus}")
            DATA_STORE[bus] = None
            continue

        fee_levels = response_json["result"]["priorityFeeLevels"]
        DATA_STORE[bus] = (fee_levels["medium"] + fee_levels["high"]) / 2
        DATA_STORE[bus] = int(DATA_STORE[bus])

        # если DATA_STORE[bus] > 10_000_000 то использовать 10_000_000
        DATA_STORE[bus] = min(DATA_STORE[bus], 10_000_000)
 

@app.get("/")
async def get():
    filtered_items = filter(lambda x: x[1] is not None, DATA_STORE.items())
    sorted_items = sorted(filtered_items, key=lambda x: x[1])
    return {
        "busses": [
            {
                "id": int(bus[-1]),
                "priority_fee": fee
            }
            for bus, fee in sorted_items
        ]
    }
