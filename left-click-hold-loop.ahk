

F1::ClickLoop()


ClickLoop(Interval=2000)
{

   static Toggler

   static mouseUp


   mouseUp := true
   Send {q down}

   Toggler := !Toggler

   TPer := Toggler ? Interval : "off"

   SetTimer, ClickClick, %TPer%

   return

   ClickClick:

   if(mouseUp)
   {
      Click, Down
   }
   else
   {
      Click, Up
   }
   mouseUp := !mouseUp
   return

}