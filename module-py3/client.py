# build: cargo build --release && cp ../target/release/libpybig2.so ./pybig2.so && python3 test.py
import pybig2
import time

print (pybig2.score_hand(0xF1000))
print (pybig2.score_hand(0x1000))

AC_PLAY = 0x000
AC_PASS = 0x100
AC_UPDATE = 0x800
AC_DEAL = 0x400

def action_msg_decode(data: int):
    player = data & 0x7
    turn   = (data >> 4 ) & 0x7
    action = (data & 0xF00)
    if action == AC_PLAY:
        print ("P%d: PLAY: 0x%x, TOACT: P%d" % (player, data & 0xFFFFFFFFFFFFF000, turn))
    if action == AC_PASS:
        print ("P%d: PASSED, TOACT: P%d" % (player, turn))
    if action == AC_UPDATE:
        print ("P%d: UPDATE, TOACT: P%d" % (player, turn))
    if action == AC_DEAL:
        print ("P%d: DEAL: 0x%x, TOACT: P%d" % (player, data & 0xFFFFFFFFFFFFF000, turn))

def play_a_game():
    BIG2 = pybig2.GameClient()

    BIG2.join("localhost:27191", "IamNOTaBOT!")

    while (1):
        bla = BIG2.poll()
        if bla is None:
            time.sleep(0.1)
            continue
        print ("DATA:", bla)
        action_msg_decode(bla)
        if BIG2.my_turn():
            print ("I pass!")
            BIG2.action_pass()

    #print ("Ende")

if __name__ == "__main__":
    import timeit
    setup = "from __main__ import play_a_game"
    print (timeit.timeit("play_a_game()", setup=setup, number=100000))