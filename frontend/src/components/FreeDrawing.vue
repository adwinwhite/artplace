<script setup>
import { ref, onMounted, onBeforeUpdate } from 'vue';
import * as protobuf from 'protobufjs';
import { Buffer } from 'buffer';
import Long from "long";

let shareUrl = ref('');
let myurl = null;

let myroom = ref('');
const can = ref(null);
let ctx = null;

let initDone = false;
let isMouseDown = false;
/** @type {WebSocket | null} */
let socket = null;
let myid = null;
let points = [];

// id -> brushKind.
let playersBrushes = new Map();
let prevPoints = new Map();
let snapper = null;
let nextLogIndex = null;

let brushStack = [];

let ClientMessage = null;
let ServerMessage = null;
let RoomInit = null;
let Snapshot = null;
// let Uid = null;
/* let JoinRoom = null; */
let Brush = null;
let PencilBrush = null;
let SetBrush = null;
let Movement = null;
let Pos = null;

function createDefaultBrush() {
  const brush = {
    width: 1,
    pencil: {
      color: "#000000"
    }
  }
  return brush;
}

function getBrush(id) {
  let brush = playersBrushes.get(id);
  if (!brush) {
    brush = createDefaultBrush();
    playersBrushes.set(id, brush);
  }
  return brush;
}


function onBrushColorChange(event) {
  const brush = {
    width: getBrush(myid).width,
    pencil: {
      color: event.target.value,
    },
  };

  sendBrush(brush);

  playersBrushes.set(myid, brush);
}

function onBrushWidthChange(event) {
  const brush = {
    width: event.target.valueAsNumber,
    pencil: {
      color: getBrush(myid).pencil.color,
    },
  };

  sendBrush(brush);

  playersBrushes.set(myid, brush);
}

function joinRoom(room) {
  console.log('joining room: ', room);
  initDone = false;
  const joinRoom = {
    roomId: room
  };
  const clientMessage = {
    joinRoom: joinRoom,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
}

function onRoomChange(event) {
  joinRoom(event.target.value)
}

function connectWs() {
  // const { location } = window

  // const proto = location.protocol.startsWith('https') ? 'wss' : 'ws' 
  // const wsUri = `${proto}://${location.host}/ws` 
  const wsUri = 'wss://www.adwin.icu/ws';

  console.log('Connecting...');
  socket = new WebSocket(wsUri);

  socket.onopen = () => {
    console.log('Connected');
  };

  socket.onmessage = async (ev) => {
    ev.data
      .arrayBuffer()
      .then((buf) => new Uint8Array(buf))
      .then((bytes) => {
        const serverMsg = ServerMessage.decode(bytes);
        /* console.log('Received client message: '); */
        /* console.log(clientMsg); */
        handleServerMessage(serverMsg);
      })
      .catch((error) => {
        // ...handle/report error...
        console.log(error);
      });
  };

  socket.onerror = (e) => {
    console.log('WebSocket error: ', e);
    socket = null;
  };

  socket.onclose = (e) => {
    console.log('Disconnected');
    console.log('Close event: ', e);
    /* socket = null; */
  };
}

function initProtobuf() {
  protobuf.util.Long = Long;
  protobuf.configure();
  let protoFile = './messages.proto';
  protobuf.load(protoFile, function (err, root) {
    if (err) throw err;

    // Obtain a message type
    ClientMessage = root.lookupType('artplace.wsmsg.ClientMessage');
    ServerMessage = root.lookupType('artplace.wsmsg.ServerMessage');
    RoomInit = root.lookupType('artplace.wsmsg.RoomInit');
    Snapshot = root.lookupType('artplace.wsmsg.Snapshot');
    // Uid = root.lookupType('artplace.wsmsg.Uid');
    Brush = root.lookupType('artplace.wsmsg.Brush');
    PencilBrush = root.lookupType('artplace.wsmsg.PencilBrush');
    SetBrush = root.lookupType('artplace.wsmsg.SetBrush');
    Movement = root.lookupType('artplace.wsmsg.Movement');
  });
}

function setBrush(canvasContext, brush) {
  if (brush.pencil) {
    canvasContext.lineWidth = brush.width;
    canvasContext.lineCap = 'round';
    canvasContext.strokeStyle = brush.pencil.color;
  }
}


function pointDrawer(movement) {
  if (movement.kind == 1 || movement.kind == 2) {
    // pointer:move or up
    ctx.save();

    const brush = getBrush(movement.id);
    setBrush(ctx, brush);
    const prevPoint = prevPoints.get(movement.id);
    ctx.beginPath();
    ctx.moveTo(prevPoint.x, prevPoint.y);
    ctx.lineTo(movement.point.x, movement.point.y);
    ctx.stroke();

    ctx.restore();
  } else if (movement.kind == 0) {
    // pointer:down
    // support drawing single dot 
  }
  prevPoints.set(movement.id, movement.point);
}

function applySnapshot(snapshot) {
  const u8Data = new Uint8ClampedArray(snapshot.bitmap.buffer);
  const imageData = new ImageData(u8Data, ctx.canvas.width, ctx.canvas.height);
  ctx.putImageData(imageData, 0, 0);

  playersBrushes = snapshot.brushes;
  prevPoints = snapshot.prevPoints;
  if (snapshot.snapper) {
    snapper = snapshot.snapper;
  }
  nextLogIndex = snapshot.nextLogIndex;
}

function createSnapshot() {
  const u8Data = new Uint8Array(ctx.getImageData(0, 0, ctx.canvas.width, ctx.canvas.height).data.buffer);
  console.log('size of canvas: ' ,u8Data.byteLength);
  const snapshot = {
    bitmap: u8Data,
    prevPoints: prevPoints,
    snapper: snapper,
    nextLogIndex: nextLogIndex
  };
  return snapshot;
}


function tryBeSnapper() {
  if (!snapper) {
    const clientMessage = {
      snapperRequest: {}
    };
    socket.send(
      ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
    );
  }
}

function increaseLogIndexAndCheckSnapshot() {
  /* console.log(nextLogIndex); */
  nextLogIndex++;
  // Check whether do I need to snapshot.
  // Need to calculate a proper number.
  // BUG: websocket met payload size limit of 64KiB.
  /* if (snapper == myid && nextLogIndex % 1000 == 0) { */
    /* console.log("Creating snapshot") */
    /* const snapshot = createSnapshot(); */
    /* const clientMessage = { */
      /* snapshot: snapshot */
    /* }; */
    /* const payload = ClientMessage.encode(ClientMessage.create(clientMessage)).finish(); */
    /* console.log('snapshot size in bytes: ', payload.byteLength); */
    /* socket.send(payload); */
    /* console.log("Snapshot sent") */
  /* } */
}

function handleServerMessage(msg) {
  if (msg.roomInit) {
    console.log(msg.roomInit);
    // Clear the canvas
    ctx.clearRect(0, 0, can.value.width, can.value.height);
    // Rerender?

    // Init room state.
    playersBrushes = new Map();
    prevPoints = new Map();
    snapper = null;
    nextLogIndex = 0;
    myroom.value = msg.roomInit.roomId;
    if (msg.roomInit.snapshot) {
      applySnapshot(msg.roomInit.snapshot);
    }
    msg.roomInit.log.forEach((m) => handleServerMessage(m));

    // set my brush for this room
    sendBrush(getBrush(myid));

    // Update url.
    myurl.searchParams.set('room', msg.roomInit.roomId);
    window.history.pushState(null, '', myurl.toString());
    shareUrl.value = window.location.href

    // after initialization, allow drawing.
    initDone = true;
  } else if (msg.setBrush) {
    playersBrushes.set(msg.setBrush.id, msg.setBrush.brush);
    increaseLogIndexAndCheckSnapshot();
  } else if (msg.movement) {
    // use local rendering to reduce tangible latency. (Will cause conflicts, different results shown to different drawers).
    if (myid != msg.movement.id) {
      pointDrawer(msg.movement);
    }
    increaseLogIndexAndCheckSnapshot();
  } else if (msg.joinRoom) {
    increaseLogIndexAndCheckSnapshot();
  } else if (msg.setSnapper) {
    snapper = msg.setSnapper.id;
    increaseLogIndexAndCheckSnapshot();
  } else if (msg.setId) {
    myid = msg.setId.id;
    console.log('I have id: ' + myid);

    // Join a room
    myurl = new URL(window.location);
    let room = null;
    if (myurl.searchParams.has('room')) {
      room = myurl.searchParams.get('room');
    }
    joinRoom(room);
  } else if (msg.leaveRoom) {
    console.log(msg.leaveRoom.id, 'left room');
    if (snapper == msg.leaveRoom.id) {
      snapper = null;
    }
    increaseLogIndexAndCheckSnapshot();
  } else if (msg.serverError) {
    console.log('server has error: ', msg.serverError);
  }
}

function pointerEvent2movement(event) {
  let kind = null;
  if (event.type == 'pointerdown') {
    kind = 0;
  } else if (event.type == 'pointermove') {
    kind = 1;
  } else if (event.type == 'pointerup') {
    kind = 2;
  }
  const movement = {
    point: {
      x: event.offsetX,
      y: event.offsetY
    },
    kind: kind
  };
  return movement;
}

function sendBrush(brush) {
  const clientMessage = {
    setBrush: {
      brush: brush
    }
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
}

function sendMovement(movement) {
  tryBeSnapper();
  const clientMessage = {
    movement: movement
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
}

function onPointerDown(event) {
  /* console.log(event); */
  if (initDone) {
    isMouseDown = true;

    // send message
    const movement = pointerEvent2movement(event);
    sendMovement(movement);
    pointDrawer(movement);
  }
}

function onPointerMove(event) {
  if (initDone && isMouseDown) {
    // send message
    const movement = pointerEvent2movement(event);
    sendMovement(movement);
    pointDrawer(movement);
  }
}

function onPointerUp(event) {
  if (initDone && isMouseDown) {
    // send message
    const movement = pointerEvent2movement(event);
    sendMovement(movement);
    pointDrawer(movement);
  }
  isMouseDown = false;
}

onMounted(() => {
  can.value.width = 400;
  can.value.height = 400;
  ctx = can.value.getContext('2d');

  can.value.addEventListener("touchmove", (e)=>{ e.preventDefault(); }, {passive: false});
  can.value.addEventListener("touchstart", (e)=>{ e.preventDefault(); }, {passive: false});
  // // init protobuf and websocket.
  initProtobuf();
  connectWs();
});

onBeforeUpdate(() => {
  if (!socket) {
    connectWs();
  }
});

</script>

<template>
  <p>Can you see me?</p>
  <form onsubmit="return false">
    <input type="color"  @change="onBrushColorChange" />
    <label for="brush-width">Brush Width:</label>
    <input
      id="brush-width"
      type="range"
      min="1"
      max="100"
      value="1"
      @change="onBrushWidthChange"
    />
    <label for="room">Room:</label>
    <input
      id="room"
      type="text"
      :placeholder="myroom"
      maxlength="16"
      @change="onRoomChange"
    />
  </form>
  <canvas ref="can" 
    @pointerdown="onPointerDown" 
    @pointermove="onPointerMove" 
    @pointerup="onPointerUp" 
    @pointerout="onPointerUp" 
    style="border: 1px solid #ccc">
  </canvas>
  <p>{{ shareUrl }} </p>
  <p>Where is my canvas?</p>
</template>

<style scoped></style>
