<script setup>
import { ref, onMounted, onBeforeUpdate } from 'vue';
import * as protobuf from 'protobufjs';
import { Buffer } from 'buffer';

let myroom = ref('');
const can = ref(null);
let ctx = null;

let initDone = false;
let isMouseDown = false;
/** @type {WebSocket | null} */
let socket = null;
let myid = null;
/* let myroom = null; */
let points = [];

// id -> brushKind.
let playersBrushes = new Map();
const prevPoints = new Map();
let brushStack = [];

let ClientMessage = null;
let InitClient = null;
// let Uid = null;
let JoinRoom = null;
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

  const setBrush = {
    id: myid,
    brush: brush
  };
  const clientMessage = {
    setBrush: setBrush,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );

  playersBrushes.set(myid, brush);
}

function onBrushWidthChange(event) {
  const brush = {
    width: event.target.valueAsNumber,
    pencil: {
      color: getBrush(myid).pencil.color,
    },
  };

  const setBrush = {
    id: myid,
    brush: brush
  };
  const clientMessage = {
    setBrush: setBrush,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );

  playersBrushes.set(myid, brush);
}

function onRoomChange(event) {
  const joinRoom = {
    id: myid,
    room: event.target.value,
  };
  const clientMessage = {
    joinRoom: joinRoom,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
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
        const clientMsg = ClientMessage.decode(bytes);
        console.log('Received client message: ');
        console.log(clientMsg);
        handleClientMessage(clientMsg);
      })
      .catch((error) => {
        // ...handle/report error...
        console.log(error);
      });
  };

  socket.onclose = () => {
    console.log('Disconnected');
    socket = null;
  };
}

function initProtobuf() {
  let protoFile = './messages.proto';
  protobuf.load(protoFile, function (err, root) {
    if (err) throw err;

    // Obtain a message type
    ClientMessage = root.lookupType('artplace.messages.ClientMessage');
    InitClient = root.lookupType('artplace.messages.InitClient');
    // Uid = root.lookupType('artplace.messages.Uid');
    Brush = root.lookupType('artplace.messages.Brush');
    PencilBrush = root.lookupType('artplace.messages.PencilBrush');
    SetBrush = root.lookupType('artplace.messages.SetBrush');
    Movement = root.lookupType('artplace.messages.Movement');
    JoinRoom = root.lookupType('artplace.messages.JoinRoom');
    Pos = root.lookupType('artplace.messages.Movement.Pos');
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

function handleClientMessage(msg) {
  if (msg.initClient) {
    // Clear the canvas
    ctx.clearRect(0, 0, can.value.width, can.value.height);
    // Rerender?

    playersBrushes = new Map();
    myid = msg.initClient.id;
    myroom.value = msg.initClient.room;
    console.log('I have id: ', myid);
    msg.initClient.messages.forEach((m) => handleClientMessage(m));

    // set my brush for this room
    const setBrush = {
      id: myid,
      brush: getBrush(myid)
    };
    const clientMessage = {
      setBrush: setBrush,
    };
    socket.send(
      ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
    );

    // after initialization, allow drawing.
    initDone = true;
  } else if (msg.setBrush) {
    playersBrushes.set(msg.setBrush.id, msg.setBrush.brush);
  } else if (msg.movement) {
    // use local rendering to reduce tangible latency. (Will cause conflicts, different results shown to different drawers).
    if (myid != msg.movement.id) {
      pointDrawer(msg.movement);
    }
  } else if (msg.joinRoom) {
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
    id: myid,
    point: {
      x: event.offsetX,
      y: event.offsetY
    },
    kind: kind
  };
  return movement;
}

function sendMovement(movement) {
  const clientMessage = {
    movement: movement
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
}

function onPointerDown(event) {
  console.log(event);
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
    style="border: 1px solid #ccc">
  </canvas>
  <p>Where is my canvas?</p>
</template>

<style scoped></style>
